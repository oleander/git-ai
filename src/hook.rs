
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::fs::File;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;

use git2::{Diff, DiffFormat, DiffOptions, Repository, Tree};
use anyhow::{Context, Result};
use thiserror::Error;
use num_cpus;

use crate::model::Model;
use crate::profile;

// Constants

const DEFAULT_STRING_CAPACITY: usize = 8192;
const PARALLEL_CHUNK_SIZE: usize = 25;
const ESTIMATED_FILES_COUNT: usize = 100;

// Types
type DiffData = Vec<(PathBuf, String, usize)>;

// Error definitions
#[derive(Error, Debug)]
pub enum HookError {
  #[error("Failed to open repository")]
  OpenRepository,

  #[error("Failed to get patch")]
  GetPatch,

  #[error("Empty diff output")]
  EmptyDiffOutput,

  #[error("Failed to write commit message")]
  WriteCommitMessage,

  #[error(transparent)]
  Anyhow(#[from] anyhow::Error)
}





// File operations traits
pub trait FilePath {
  fn is_empty(&self) -> Result<bool> {
    self.read().map(|s| s.is_empty())
  }

  fn write(&self, msg: String) -> Result<()>;
  fn read(&self) -> Result<String>;
}

impl FilePath for PathBuf {
  fn write(&self, msg: String) -> Result<()> {
    File::create(self)?
      .write_all(msg.as_bytes())
      .map_err(Into::into)
  }

  fn read(&self) -> Result<String> {
    let mut contents = String::new();
    File::open(self)?.read_to_string(&mut contents)?;
    Ok(contents)
  }
}

// Git operations traits
trait DiffDeltaPath {
  fn path(&self) -> PathBuf;
}

impl DiffDeltaPath for git2::DiffDelta<'_> {
  fn path(&self) -> PathBuf {
    self
      .new_file()
      .path()
      .or_else(|| self.old_file().path())
      .map(PathBuf::from)
      .unwrap_or_default()
  }
}

// String conversion traits
pub trait Utf8String {
  fn to_utf8(&self) -> String;
}

impl Utf8String for Vec<u8> {
  fn to_utf8(&self) -> String {
    // Fast path for valid UTF-8 (most common case)
    if let Ok(s) = std::str::from_utf8(self) {
      return s.to_string();
    }
    // Fallback for invalid UTF-8
    String::from_utf8_lossy(self).into_owned()
  }
}

impl Utf8String for [u8] {
  fn to_utf8(&self) -> String {
    // Fast path for valid UTF-8 (most common case)
    if let Ok(s) = std::str::from_utf8(self) {
      return s.to_string();
    }
    // Fallback for invalid UTF-8
    String::from_utf8_lossy(self).into_owned()
  }
}

// Patch generation traits
pub trait PatchDiff {
  fn to_patch(&self, max_token_count: usize, model: Model) -> Result<String>;
  fn collect_diff_data(&self) -> Result<HashMap<PathBuf, String>>;
  fn is_empty(&self) -> Result<bool>;
}

impl PatchDiff for Diff<'_> {
  fn to_patch(&self, max_tokens: usize, model: Model) -> Result<String> {
    profile!("Generating patch diff");

    // Step 1: Collect diff data (non-parallel)
    let files = self.collect_diff_data()?;
    if files.is_empty() {
      return Ok(String::new());
    }

    // Fast path for small diffs - skip tokenization entirely
    if files.len() == 1 {
      profile!("Single file fast path");
      let (_, content) = files
        .into_iter()
        .next()
        .ok_or_else(|| HookError::EmptyDiffOutput)?;

      // If content is small enough to fit, just return it directly
      if content.len() < max_tokens * 4 {
        // Estimate 4 chars per token
        return Ok(content);
      }

      // Otherwise do a simple truncation
      return model.truncate(&content, max_tokens);
    }

    // Optimization: Skip token counting entirely for small diffs
    if files.len() <= 5 && max_tokens > 500 {
      profile!("Small diff fast path");
      let mut result = String::new();
      let files_clone = files.clone(); // Clone files for use after iteration

      // Just combine the files with a limit on total size
      for (i, (_, content)) in files.into_iter().enumerate() {
        if i > 0 {
          result.push('\n');
        }
        // Only add as much as we can estimate will fit
        let limit = (max_tokens / files_clone.len()) * 4; // ~4 chars per token
        let truncated = if content.len() > limit {
          let truncated = content.chars().take(limit).collect::<String>();
          // Find last space to avoid cutting words
          let last_space = truncated
            .rfind(char::is_whitespace)
            .unwrap_or(truncated.len());
          if last_space > 0 {
            truncated[..last_space].to_string()
          } else {
            truncated
          }
        } else {
          content
        };
        result.push_str(&truncated);
      }

      return Ok(result);
    }

    // Step 2: Prepare files for processing - optimized path for medium diffs
    if files.len() <= 20 {
      profile!("Medium diff optimized path");

      // Convert to vector with simple heuristic for token count
      let mut files_vec: Vec<(PathBuf, String, usize)> = files
        .into_iter()
        .map(|(path, content)| {
          // Estimate token count as character count / 4
          let estimated_tokens = content.len() / 4;
          (path, content, estimated_tokens)
        })
        .collect();

      // Sort by estimated size
      files_vec.sort_by_key(|(_, _, count)| *count);

      // Allocate tokens to files and process
      let mut result = String::new();
      let mut tokens_used = 0;

      for (i, (_, content, estimated_tokens)) in files_vec.into_iter().enumerate() {
        if tokens_used >= max_tokens {
          break;
        }

        if i > 0 {
          result.push('\n');
        }

        let tokens_left = max_tokens.saturating_sub(tokens_used);
        let tokens_for_file = estimated_tokens.min(tokens_left);

        // Only truncate if needed
        let processed_content = if estimated_tokens > tokens_for_file {
          // Simple character-based truncation for speed
          let char_limit = tokens_for_file * 4;
          let truncated: String = content.chars().take(char_limit).collect();
          truncated
        } else {
          content
        };

        result.push_str(&processed_content);
        tokens_used += tokens_for_file;
      }

      return Ok(result);
    }

    // Step 3: Complex diff path - use parallel processing with optimizations
    profile!("Converting files to vector");
    let files_vec: Vec<_> = files.into_iter().collect();
    let total_files = files_vec.len();

    // Use rayon for parallel token counting - with batching for performance
    let thread_pool = rayon::ThreadPoolBuilder::new()
      .num_threads(num_cpus::get())
      .build()
      .context("Failed to create thread pool")?;

    profile!("Parallel token counting");
    // Use chunked processing for token counting to reduce contention
    let chunk_size = (total_files / num_cpus::get().max(1)).max(10);
    let files_with_tokens: DiffData = thread_pool.install(|| {
      files_vec
        .chunks(chunk_size)
        .collect::<Vec<_>>()
        .into_par_iter()
        .flat_map(|chunk| {
          chunk
            .iter()
            .map(|(path, content)| {
              let token_count = model.count_tokens(content).unwrap_or_default();
              (path.clone(), content.clone(), token_count)
            })
            .collect::<Vec<_>>()
        })
        .collect()
    });

    // Skip sorting for very large diffs - it's not worth the time
    profile!("Sorting files by token count");
    let sorted_files = if total_files > 500 {
      files_with_tokens
    } else {
      let mut sorted = files_with_tokens;
      sorted.sort_by_key(|(_, _, count)| *count);
      sorted
    };

    // Step 4: Process files with optimized token allocation
    let remaining_tokens = Arc::new(AtomicUsize::new(max_tokens));
    let results = Arc::new(parking_lot::RwLock::new(Vec::with_capacity(total_files)));
    let processed_files = Arc::new(AtomicUsize::new(0));

    // Optimize chunking - use larger chunks for better performance
    let adaptive_chunk_size = (total_files / (2 * num_cpus::get().max(1))).max(PARALLEL_CHUNK_SIZE);

    let chunks: Vec<_> = sorted_files
      .chunks(adaptive_chunk_size)
      .map(|chunk| chunk.to_vec())
      .collect();

    let model = Arc::new(model);

    profile!("Parallel chunk processing");
    thread_pool.install(|| {
      chunks
        .par_iter()
        .try_for_each(|chunk| process_chunk(chunk, &model, total_files, &processed_files, &remaining_tokens, &results))
    })?;

    // Step 5: Combine results efficiently
    profile!("Combining results");
    let results_guard = results.read();

    // Fast path for empty results
    if results_guard.is_empty() {
      return Ok(String::new());
    }

    // Optimize string allocation
    let total_len = results_guard
      .iter()
      .map(|(_, content): &(PathBuf, String)| content.len())
      .sum::<usize>();
    let mut final_result = String::with_capacity(total_len + results_guard.len());

    for (i, (_, content)) in results_guard.iter().enumerate() {
      if i > 0 {
        final_result.push('\n');
      }
      final_result.push_str(content);
    }

    Ok(final_result)
  }

  fn collect_diff_data(&self) -> Result<HashMap<PathBuf, String>> {
    profile!("Processing diff changes");

    // Pre-allocate HashMap with estimated capacity
    let mut files = HashMap::with_capacity(ESTIMATED_FILES_COUNT);



    // Create thread-local cache for paths to avoid allocations
    thread_local! {
      static PATH_CACHE: std::cell::RefCell<HashMap<PathBuf, ()>> =
        std::cell::RefCell::new(HashMap::with_capacity(20));
    }

    // Process diffs with optimized buffer handling
    self.print(DiffFormat::Patch, |diff, _hunk, line| {
      // Get path with potential reuse from cache for better performance
      let path = PATH_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let new_path = diff.path();
        if let Some(existing_path) = cache.keys().find(|p| *p == &new_path) {
          existing_path.clone()
        } else {
          cache.insert(new_path.clone(), ());
          new_path
        }
      });

      // Fast path for UTF-8 content - avoid expensive conversions
      let content = if let Ok(s) = std::str::from_utf8(line.content()) {
        s.to_string()
      } else {
        // Fallback for non-UTF8 content
        line.content().to_utf8()
      };

      // Process line by line origin more efficiently
      match line.origin() {
        '+' | '-' | ' ' => {
          // Added, removed, or context lines - append with origin prefix
          let entry = files
            .entry(path)
            .or_insert_with(|| String::with_capacity(DEFAULT_STRING_CAPACITY));

          // Add the origin character for proper diff format
          match line.origin() {
            '+' => entry.push('+'),
            '-' => entry.push('-'),
            ' ' => entry.push(' '),
            _ => {}
          }
          entry.push_str(&content);
        }
        _ => {
          // Other lines (headers, etc.) - skip them as they're not part of the actual diff content
          // The git diff headers are already included by the diff format
          log::trace!("Skipping diff line with origin: {:?}", line.origin());
        }
      }

      true
    })?;

    Ok(files)
  }

  fn is_empty(&self) -> Result<bool> {
    let mut has_changes = false;

    self.foreach(
      &mut |_file, _progress| {
        has_changes = true;
        true
      },
      None,
      None,
      None
    )?;

    Ok(!has_changes)
  }
}

fn process_chunk(
  chunk: &[(PathBuf, String, usize)], model: &Arc<Model>, total_files: usize, processed_files: &AtomicUsize,
  remaining_tokens: &AtomicUsize, result_chunks: &Arc<parking_lot::RwLock<Vec<(PathBuf, String)>>>
) -> Result<()> {
  profile!("Processing chunk");
  // Fast path for empty chunks
  if chunk.is_empty() {
    return Ok(());
  }

  // Fast path for no tokens remaining
  let total_remaining = remaining_tokens.load(Ordering::Acquire);
  if total_remaining == 0 {
    return Ok(());
  }

  // Ultra-fast path for small chunks that will likely fit
  if chunk.len() <= 3 {
    let total_token_count = chunk.iter().map(|(_, _, count)| *count).sum::<usize>();
    // If entire chunk is small enough, process it in one go
    if total_token_count <= total_remaining {
      // Try to allocate all tokens at once
      if remaining_tokens
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
          if current >= total_token_count {
            Some(current - total_token_count)
          } else {
            None
          }
        })
        .is_ok()
      {
        // Update processed files counter once
        processed_files.fetch_add(chunk.len(), Ordering::AcqRel);

        // Collect all results without truncation
        let chunk_results: Vec<_> = chunk
          .iter()
          .map(|(path, content, _)| (path.clone(), content.clone()))
          .collect();

        if !chunk_results.is_empty() {
          result_chunks.write().extend(chunk_results);
        }

        return Ok(());
      }
    }
  }

  // Fast path for small files that don't need tokenization
  let mut chunk_results = Vec::with_capacity(chunk.len());
  let mut local_processed = 0;

  for (path, content, token_count) in chunk {
    local_processed += 1;

    // Recheck remaining tokens to allow early exit
    let current_remaining = remaining_tokens.load(Ordering::Acquire);
    if current_remaining == 0 {
      break;
    }

    // For very small files or text, don't bother with complex calculations
    let token_count = *token_count;

    // If small content is less than threshold, just clone without tokenization
    if token_count <= 100
      && token_count <= current_remaining
      && remaining_tokens
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
          if current >= token_count {
            Some(current - token_count)
          } else {
            None
          }
        })
        .is_ok()
    {
      chunk_results.push((path.clone(), content.clone()));
      continue;
    }

    // For larger content, do the normal allocation
    // Batch update processed files counter - just once at the end
    let current_file_num = processed_files.load(Ordering::Acquire);
    let files_remaining = total_files.saturating_sub(current_file_num + local_processed);

    // Calculate tokens per file
    let max_tokens_per_file = if files_remaining > 0 {
      current_remaining.saturating_div(files_remaining)
    } else {
      current_remaining
    };

    if max_tokens_per_file == 0 {
      continue;
    }

    let allocated_tokens = token_count.min(max_tokens_per_file);

    if remaining_tokens
      .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
        if current >= allocated_tokens {
          Some(current - allocated_tokens)
        } else {
          None
        }
      })
      .is_ok()
    {
      // Fast path for content that doesn't need truncation
      if token_count <= allocated_tokens {
        chunk_results.push((path.clone(), content.clone()));
      } else {
        // Use fast character-based truncation for most cases
        if content.len() < 2000 || allocated_tokens > 500 {
          // Character-based truncation is much faster than tokenization
          let char_limit = allocated_tokens * 4;
          let truncated: String = content.chars().take(char_limit).collect();
          chunk_results.push((path.clone(), truncated));
        } else {
          // Use proper truncation for complex cases
          let truncated = model.truncate(content, allocated_tokens)?;
          chunk_results.push((path.clone(), truncated));
        }
      }
    }
  }

  // Update processed files counter once at the end
  if local_processed > 0 {
    processed_files.fetch_add(local_processed, Ordering::AcqRel);
  }

  // Batch update the result collection
  if !chunk_results.is_empty() {
    result_chunks.write().extend(chunk_results);
  }

  Ok(())
}

pub trait PatchRepository {
  fn to_patch(&self, tree: Option<Tree<'_>>, max_token_count: usize, model: Model) -> Result<String>;
  fn to_diff(&self, tree: Option<Tree<'_>>) -> Result<git2::Diff<'_>>;
  fn to_commit_diff(&self, tree: Option<Tree<'_>>) -> Result<git2::Diff<'_>>;
  fn configure_diff_options(&self, opts: &mut DiffOptions);
  fn configure_commit_diff_options(&self, opts: &mut DiffOptions);
}

impl PatchRepository for Repository {
  fn to_patch(&self, tree: Option<Tree>, max_token_count: usize, model: Model) -> Result<String> {
    profile!("Repository patch generation");
    self.to_commit_diff(tree)?.to_patch(max_token_count, model)
  }

  fn to_diff(&self, tree: Option<Tree<'_>>) -> Result<git2::Diff<'_>> {
    profile!("Git diff generation");
    let mut opts = DiffOptions::new();
    self.configure_diff_options(&mut opts);

    match tree {
      Some(tree) => {
        // Get the diff between tree and working directory, including staged changes
        self.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts))
      }
      None => {
        // If there's no HEAD yet, compare against an empty tree
        let empty_tree = self.find_tree(self.treebuilder(None)?.write()?)?;
        // Get the diff between empty tree and working directory, including staged changes
        self.diff_tree_to_workdir_with_index(Some(&empty_tree), Some(&mut opts))
      }
    }
    .context("Failed to get diff")
  }

  fn to_commit_diff(&self, tree: Option<Tree<'_>>) -> Result<git2::Diff<'_>> {
    profile!("Git commit diff generation");
    let mut opts = DiffOptions::new();
    self.configure_commit_diff_options(&mut opts);

    match tree {
      Some(tree) => {
        // Get the diff between tree and index (staged changes only)
        self.diff_tree_to_index(Some(&tree), None, Some(&mut opts))
      }
      None => {
        // If there's no HEAD yet, compare against an empty tree
        let empty_tree = self.find_tree(self.treebuilder(None)?.write()?)?;
        // Get the diff between empty tree and index (staged changes only)
        self.diff_tree_to_index(Some(&empty_tree), None, Some(&mut opts))
      }
    }
    .context("Failed to get diff")
  }

  fn configure_diff_options(&self, opts: &mut DiffOptions) {
    opts
      .ignore_whitespace_change(true)
      .recurse_untracked_dirs(true)
      .recurse_ignored_dirs(false)
      .ignore_whitespace_eol(true)
      .ignore_blank_lines(true)
      .include_untracked(true)
      .ignore_whitespace(true)
      .indent_heuristic(false)
      .ignore_submodules(true)
      .include_ignored(false)
      .interhunk_lines(0)
      .context_lines(0)
      .patience(true)
      .minimal(true);
  }

  fn configure_commit_diff_options(&self, opts: &mut DiffOptions) {
    opts
      .ignore_whitespace_change(false)
      .recurse_untracked_dirs(false)
      .recurse_ignored_dirs(false)
      .ignore_whitespace_eol(true)
      .ignore_blank_lines(true)
      .include_untracked(false)
      .ignore_whitespace(true)
      .indent_heuristic(false)
      .ignore_submodules(true)
      .include_ignored(false)
      .interhunk_lines(0)
      .context_lines(0)
      .patience(true)
      .minimal(true);
  }
}




