use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::fs::File;
use std::sync::Arc;

use rayon::prelude::*;
use git2::{Diff, DiffFormat, DiffOptions, Repository, Tree};
use anyhow::{Context, Result};
use thiserror::Error;
use num_cpus;

use crate::model::Model;
use crate::profile;

// Constants

const DEFAULT_STRING_CAPACITY: usize = 8192;
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

    // Step 1: Collect diff data (non-parallel).
    //
    // `collect_diff_data` returns a `HashMap`, whose iteration order is randomized per
    // instance (std `RandomState`). Iterating it directly made the combined patch's file
    // order non-deterministic, so identical staged input could yield different patches and
    // therefore different commit messages. We sort by path *once* here and feed the ordered
    // `Vec` to every downstream path, making all of them deterministic. (C4)
    let files = self.collect_diff_data()?;
    if files.is_empty() {
      return Ok(String::new());
    }

    let mut files: Vec<(PathBuf, String)> = files.into_iter().collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));

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
      let file_count = files.len(); // Capture before consuming `files`.
      let mut result = String::new();

      // Just combine the files with a limit on total size
      for (i, (_, content)) in files.into_iter().enumerate() {
        if i > 0 {
          result.push('\n');
        }
        // Only add as much as we can estimate will fit
        let limit = (max_tokens / file_count) * 4; // ~4 chars per token
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

    // Step 2: Prepare files for processing - optimized path for medium diffs.
    //
    // Files are already in deterministic path order. We keep a stable secondary sort by
    // estimated token count (smaller files first, to fit as many whole files as possible)
    // while preserving path order among equal-sized files via `sort_by_key`'s stability.
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

      // Stable sort by estimated size (path order preserved among ties => deterministic).
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

    // Step 3: Complex diff path - parallel processing with deterministic output.
    //
    // CPU-bound work (token counting, truncation) runs on rayon. The previous design used a
    // shared atomic token budget plus a shared result `Vec` written from `try_for_each`, which
    // made both the *truncation* (budget races) and the *output order* (completion order)
    // non-deterministic. We replace that with:
    //   1. order-preserving parallel token counting (`par_iter().map().collect()`),
    //   2. a sequential budget pass over the path-sorted files (cheap arithmetic),
    //   3. order-preserving parallel truncation,
    // so identical input always yields byte-identical output. (C3b + C4)
    profile!("Converting files to vector");
    let total_files = files.len();

    let thread_pool = rayon::ThreadPoolBuilder::new()
      .num_threads(num_cpus::get())
      .build()
      .context("Failed to create thread pool")?;

    // Token counting (CPU-bound, parallel, order-preserving).
    profile!("Parallel token counting");
    let files_with_tokens: DiffData = thread_pool.install(|| {
      files
        .par_iter()
        .map(|(path, content)| {
          let token_count = model.count_tokens(content).unwrap_or_default();
          (path.clone(), content.clone(), token_count)
        })
        .collect()
    });

    // Stable sort by token count (smaller first), preserving path order among ties. For very
    // large diffs we skip the sort but keep the deterministic path order from Step 1.
    profile!("Sorting files by token count");
    let sorted_files = if total_files > 500 {
      files_with_tokens
    } else {
      let mut sorted = files_with_tokens;
      sorted.sort_by_key(|(_, _, count)| *count);
      sorted
    };

    // Step 4: Sequential budget allocation (deterministic). Decide how many tokens each file
    // may keep, in order, with no cross-thread races. We carry the already-computed
    // `token_count` forward so the truncation pass does not recount.
    profile!("Allocating token budget");
    let mut remaining = max_tokens;
    // (path, content, token_count, allocated)
    let mut allocations: Vec<(PathBuf, String, usize, usize)> = Vec::with_capacity(sorted_files.len());
    let mut files_left = sorted_files.len();
    for (path, content, token_count) in sorted_files.into_iter() {
      if remaining == 0 {
        break;
      }
      // Even share of the remaining budget, capped at what the file actually needs.
      let fair_share = remaining / files_left.max(1);
      let allocated = token_count.min(fair_share.max(1)).min(remaining);
      remaining -= allocated;
      files_left = files_left.saturating_sub(1);
      allocations.push((path, content, token_count, allocated));
    }

    // Truncation (CPU-bound, parallel, order-preserving). Errors propagate rather than being
    // silently swallowed into an empty file.
    profile!("Parallel truncation");
    let model = Arc::new(model);
    let processed: Vec<(PathBuf, String)> = thread_pool.install(|| {
      allocations
        .par_iter()
        .map(|(path, content, token_count, allocated)| {
          let out = if *token_count <= *allocated {
            content.clone()
          } else if content.len() < 2000 || *allocated > 500 {
            // Character-based truncation is much faster than tokenization.
            content.chars().take(allocated * 4).collect::<String>()
          } else {
            model.truncate(content, *allocated)?
          };
          Ok((path.clone(), out))
        })
        .collect::<Result<Vec<_>>>()
    })?;

    // Step 5: Combine results in the (deterministic) order produced above.
    profile!("Combining results");
    if processed.is_empty() {
      return Ok(String::new());
    }

    let total_len = processed
      .iter()
      .map(|(_, content)| content.len())
      .sum::<usize>();
    let mut final_result = String::with_capacity(total_len + processed.len());

    for (i, (_, content)) in processed.iter().enumerate() {
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
        'F' | 'H' => {
          // File headers (diff --git, index, ---, +++) and hunk headers (@@ ... @@) carry the
          // structure parse_diff() needs to split the patch into per-file sections. Dropping them
          // (the v1.1.1 regression) collapsed every diff into a single "unknown" file.
          let entry = files
            .entry(path)
            .or_insert_with(|| String::with_capacity(DEFAULT_STRING_CAPACITY));
          entry.push_str(&content);
        }
        _ => {
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
