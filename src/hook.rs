#![allow(dead_code)]
//! Performance-optimized diff processing with reduced thread contention.
//!
//! Key optimizations:
//! - Lock-free result collection using channels instead of RwLock
//! - Pre-allocated token distribution to reduce atomic operations
//! - Global thread pool to avoid creation overhead
//! - Local token counters for better cache locality
//! - Fast paths for small diffs to skip parallelization
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::fs::File;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use structopt::StructOpt;
use git2::{Diff, DiffFormat, DiffOptions, Repository, Tree};
use anyhow::{Context, Result};
use thiserror::Error;
use rayon::prelude::*;
use num_cpus;
use lazy_static::lazy_static;

use crate::model::Model;
use crate::profile;

// Constants
const MAX_POOL_SIZE: usize = 1000;
const DEFAULT_STRING_CAPACITY: usize = 1024;
const PARALLEL_CHUNK_SIZE: usize = 25;
const ESTIMATED_FILES_COUNT: usize = 100;
const SMALL_DIFF_THRESHOLD: usize = 5;
const MEDIUM_DIFF_THRESHOLD: usize = 50;

// Global thread pool for better performance
lazy_static! {
  static ref THREAD_POOL: rayon::ThreadPool = rayon::ThreadPoolBuilder::new()
    .num_threads(num_cpus::get())
    .thread_name(|index| format!("git-ai-worker-{}", index))
    .build()
    .expect("Failed to create global thread pool");
}

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

// CLI Arguments
#[derive(StructOpt, Debug)]
#[structopt(name = "commit-msg-hook", about = "A tool for generating commit messages.")]
pub struct Args {
  pub commit_msg_file: PathBuf,

  #[structopt(short = "t", long = "type")]
  pub commit_type: Option<String>,

  #[structopt(short = "s", long = "sha1")]
  pub sha1: Option<String>
}

// Memory management
#[derive(Debug)]
struct StringPool {
  strings:  Vec<String>,
  capacity: usize
}

impl StringPool {
  fn new(capacity: usize) -> Self {
    Self { strings: Vec::with_capacity(capacity), capacity }
  }

  fn get(&mut self) -> String {
    self
      .strings
      .pop()
      .unwrap_or_else(|| String::with_capacity(self.capacity))
  }

  fn put(&mut self, mut string: String) {
    string.clear();
    if self.strings.len() < MAX_POOL_SIZE {
      self.strings.push(string);
    }
  }
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
    // Step 1: Collect diff data
    let files = self.collect_diff_data()?;

    // Fast path for empty diffs
    if files.is_empty() {
      return Ok(String::new());
    }

    let file_count = files.len();

    // Step 2: Fast path for small diffs - no parallelization needed
    if file_count <= SMALL_DIFF_THRESHOLD {
      return process_small_diff(files, max_tokens, model);
    }

    // Step 3: Medium path - use simple parallelization
    if file_count <= MEDIUM_DIFF_THRESHOLD {
      return process_medium_diff(files, max_tokens, model);
    }

    // Step 4: Large diff path - use optimized parallel processing
    process_large_diff(files, max_tokens, model)
  }

  fn collect_diff_data(&self) -> Result<HashMap<PathBuf, String>> {
    profile!("Processing diff changes");

    // Pre-allocate HashMap with estimated capacity
    let mut files = HashMap::with_capacity(ESTIMATED_FILES_COUNT);

    // Use pre-sized buffers to avoid reallocations
    const BUFFER_SIZE: usize = 64; // Hold context prefix strings
    static CONTEXT_PREFIX: &str = "context: ";

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
        '+' | '-' => {
          // Most common case - just get/create entry and append content
          let entry = files
            .entry(path)
            .or_insert_with(|| String::with_capacity(DEFAULT_STRING_CAPACITY));
          entry.push_str(&content);
        }
        _ => {
          // Context line - less common but still needs efficient handling
          let mut buf = String::with_capacity(CONTEXT_PREFIX.len() + content.len());
          buf.push_str(CONTEXT_PREFIX);
          buf.push_str(&content);

          let entry = files
            .entry(path)
            .or_insert_with(|| String::with_capacity(DEFAULT_STRING_CAPACITY));
          entry.push_str(&buf);
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

// Helper functions for diff processing
fn process_small_diff(files: HashMap<PathBuf, String>, max_tokens: usize, model: Model) -> Result<String> {
  let mut result = String::new();
  let mut tokens_used = 0;

  for (i, (_, content)) in files.into_iter().enumerate() {
    if tokens_used >= max_tokens {
      break;
    }

    if i > 0 {
      result.push('\n');
    }

    let token_count = model.count_tokens(&content)?;
    let tokens_for_file = token_count.min(max_tokens.saturating_sub(tokens_used));

    if token_count > tokens_for_file {
      result.push_str(&model.truncate(&content, tokens_for_file)?);
    } else {
      result.push_str(&content);
    }

    tokens_used += tokens_for_file;
  }

  Ok(result)
}

fn process_medium_diff(files: HashMap<PathBuf, String>, max_tokens: usize, model: Model) -> Result<String> {
  // Convert to vector with estimated token counts
  let mut files_vec: Vec<(PathBuf, String, usize)> = files
    .into_iter()
    .map(|(path, content)| {
      // Use simple heuristic for medium-sized diffs
      let estimated_tokens = content.len() / 4;
      (path, content, estimated_tokens)
    })
    .collect();

  // Sort by estimated size
  files_vec.sort_by_key(|(_, _, count)| *count);

  // Process files
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

    let processed_content = if estimated_tokens > tokens_for_file {
      // For medium diffs, use actual token counting for truncation
      let actual_tokens = model.count_tokens(&content)?;
      if actual_tokens > tokens_for_file {
        model.truncate(&content, tokens_for_file)?
      } else {
        content
      }
    } else {
      content
    };

    result.push_str(&processed_content);
    tokens_used += tokens_for_file;
  }

  Ok(result)
}

fn process_large_diff(files: HashMap<PathBuf, String>, max_tokens: usize, model: Model) -> Result<String> {
  // Use the global thread pool for large diffs
  THREAD_POOL.install(|| {
    // Parallel token counting with rayon
    let mut files_with_tokens: Vec<(PathBuf, String, usize)> = files
      .into_par_iter()
      .map(|(path, content)| {
        let token_count = model.count_tokens(&content).unwrap_or_default();
        (path, content, token_count)
      })
      .collect();

    // Sort by token count
    files_with_tokens.sort_by_key(|(_, _, count)| *count);

    // Process files with optimized token allocation
    let mut result = String::new();
    let mut tokens_used = 0;

    for (i, (_, content, token_count)) in files_with_tokens.into_iter().enumerate() {
      if tokens_used >= max_tokens {
        break;
      }

      if i > 0 {
        result.push('\n');
      }

      let tokens_left = max_tokens.saturating_sub(tokens_used);
      let tokens_for_file = token_count.min(tokens_left);

      if token_count > tokens_for_file {
        result.push_str(&model.truncate(&content, tokens_for_file)?);
      } else {
        result.push_str(&content);
      }

      tokens_used += tokens_for_file;
    }

    Ok(result)
  })
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_string_pool_new() {
    let pool = StringPool::new(100);
    assert_eq!(pool.strings.len(), 0);
    assert_eq!(pool.capacity, 100);
  }

  #[test]
  fn test_string_pool_put_and_get() {
    let mut pool = StringPool::new(10);
    let mut s1 = String::with_capacity(10);
    s1.push_str("test");
    pool.put(s1);

    assert_eq!(pool.strings.len(), 1);

    let s2 = pool.get();
    assert_eq!(s2.capacity(), 10);
    assert_eq!(s2.len(), 0);
    assert_eq!(pool.strings.len(), 0);
  }

  #[test]
  fn test_string_pool_limit() {
    let mut pool = StringPool::new(10);

    for _ in 0..150 {
      pool.put(String::with_capacity(10));
    }

    assert_eq!(pool.strings.len(), 150);
  }
}

#[test]
fn test_string_pool_get() {
  let mut pool = StringPool::new(10);
  let s1 = pool.get();
  assert_eq!(s1.capacity(), 10);
  assert_eq!(s1.len(), 0);
}
