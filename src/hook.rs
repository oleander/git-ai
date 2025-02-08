#![allow(dead_code)]
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs::File;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::io::{Read, Write};

use structopt::StructOpt;
use git2::{Diff, DiffFormat, DiffOptions, Repository, Tree};
use anyhow::{Context, Result};
use thiserror::Error;
use rayon::prelude::*;
use parking_lot::Mutex;
use num_cpus;

use crate::model::Model;
use crate::profile;

// Constants
const MAX_POOL_SIZE: usize = 100;
const DEFAULT_STRING_CAPACITY: usize = 4096;
const PARALLEL_CHUNK_SIZE: usize = 10;

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
    String::from_utf8_lossy(self).into_owned()
  }
}

impl Utf8String for [u8] {
  fn to_utf8(&self) -> String {
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

    // Step 2: Prepare files for processing
    let mut files_with_tokens: DiffData = files
      .into_iter()
      .map(|(path, content)| {
        let token_count = model.count_tokens(&content).unwrap_or_default();
        (path, content, token_count)
      })
      .collect();

    files_with_tokens.sort_by_key(|(_, _, count)| *count);

    // Step 3: Process files in parallel
    let thread_pool = rayon::ThreadPoolBuilder::new()
      .num_threads(num_cpus::get())
      .build()
      .context("Failed to create thread pool")?;

    let total_files = files_with_tokens.len();
    let remaining_tokens = Arc::new(AtomicUsize::new(max_tokens));
    let result_chunks = Arc::new(Mutex::new(Vec::with_capacity(total_files)));
    let processed_files = Arc::new(AtomicUsize::new(0));

    let chunks: Vec<_> = files_with_tokens
      .chunks(PARALLEL_CHUNK_SIZE)
      .map(|chunk| chunk.to_vec())
      .collect();

    let model = Arc::new(model);

    thread_pool.install(|| {
      chunks
        .par_iter()
        .try_for_each(|chunk| process_chunk(chunk, &model, total_files, &processed_files, &remaining_tokens, &result_chunks))
    })?;

    // Step 4: Combine results
    let results = result_chunks.lock();
    let mut final_result = String::with_capacity(
      results
        .iter()
        .map(|(_, content): &(PathBuf, String)| content.len())
        .sum()
    );

    for (_, content) in results.iter() {
      if !final_result.is_empty() {
        final_result.push('\n');
      }
      final_result.push_str(content);
    }

    Ok(final_result)
  }

  fn collect_diff_data(&self) -> Result<HashMap<PathBuf, String>> {
    profile!("Processing diff changes");

    let string_pool = Arc::new(Mutex::new(StringPool::new(DEFAULT_STRING_CAPACITY)));
    let files = Arc::new(Mutex::new(HashMap::new()));

    self.print(DiffFormat::Patch, |diff, _hunk, line| {
      let content = line.content().to_utf8();
      let mut line_content = string_pool.lock().get();

      match line.origin() {
        '+' | '-' => line_content.push_str(&content),
        _ => {
          line_content.push_str("context: ");
          line_content.push_str(&content);
        }
      };

      let mut files = files.lock();
      let entry = files
        .entry(diff.path())
        .or_insert_with(|| String::with_capacity(DEFAULT_STRING_CAPACITY));
      entry.push_str(&line_content);
      string_pool.lock().put(line_content);
      true
    })?;

    Ok(
      Arc::try_unwrap(files)
        .expect("Arc still has multiple owners")
        .into_inner()
    )
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
  remaining_tokens: &AtomicUsize, result_chunks: &Arc<Mutex<Vec<(PathBuf, String)>>>
) -> Result<()> {
  let mut chunk_results = Vec::with_capacity(chunk.len());

  for (path, content, token_count) in chunk {
    let current_file_num = processed_files.fetch_add(1, Ordering::SeqCst);
    let files_remaining = total_files.saturating_sub(current_file_num);

    // Calculate max_tokens_per_file based on actual remaining files
    let total_remaining = remaining_tokens.load(Ordering::SeqCst);
    let max_tokens_per_file = if files_remaining > 0 {
      total_remaining.saturating_div(files_remaining)
    } else {
      total_remaining
    };

    if max_tokens_per_file == 0 {
      continue;
    }

    let token_count = *token_count;
    let allocated_tokens = token_count.min(max_tokens_per_file);

    if remaining_tokens
      .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
        if current >= allocated_tokens {
          Some(current - allocated_tokens)
        } else {
          None
        }
      })
      .is_ok()
    {
      let processed_content = if token_count > allocated_tokens {
        model.truncate(content, allocated_tokens)?
      } else {
        content.clone()
      };
      chunk_results.push((path.clone(), processed_content));
    }
  }

  if !chunk_results.is_empty() {
    result_chunks.lock().extend(chunk_results);
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
  fn test_string_pool_get() {
    let mut pool = StringPool::new(10);
    let s1 = pool.get();
    assert_eq!(s1.capacity(), 10);
    assert_eq!(s1.len(), 0);
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

    assert_eq!(pool.strings.len(), MAX_POOL_SIZE);
  }
}
