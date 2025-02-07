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
use parking_lot::Mutex;
use num_cpus;

use crate::model::Model;
use crate::profile;

// String pool for reusing allocations
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
    if self.strings.len() < 100 {
      // Limit pool size
      self.strings.push(string);
    }
  }
}

pub trait FilePath {
  fn is_empty(&self) -> Result<bool> {
    self.read().map(|s| s.is_empty())
  }

  fn write(&self, msg: String) -> Result<()>;
  fn read(&self) -> Result<String>;
}

impl FilePath for PathBuf {
  fn write(&self, msg: String) -> Result<()> {
    let mut file = File::create(self)?;
    file.write_all(msg.as_bytes())?;
    Ok(())
  }

  fn read(&self) -> Result<String> {
    let mut file = File::open(self)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
  }
}

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

pub trait PatchDiff {
  fn to_patch(&self, max_token_count: usize, model: Model) -> Result<String>;
}

impl PatchDiff for Diff<'_> {
  fn to_patch(&self, max_tokens: usize, model: Model) -> Result<String> {
    profile!("Generating patch diff");

    // Create thread pool for parallel operations
    let thread_pool = rayon::ThreadPoolBuilder::new()
      .num_threads(num_cpus::get())
      .build()
      .unwrap();

    // Step 1: Collect all diff data into thread-safe structures
    let string_pool = Arc::new(Mutex::new(StringPool::new(4096)));
    let files = Arc::new(Mutex::new(HashMap::new()));

    {
      profile!("Processing diff changes");
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
          .or_insert_with(|| String::with_capacity(4096));
        entry.push_str(&line_content);
        string_pool.lock().put(line_content);
        true
      })?;
    }

    // Step 2: Move data out of thread-safe containers
    let files = Arc::try_unwrap(files)
      .expect("Arc still has multiple owners")
      .into_inner();

    let total_files = files.len();
    let remaining_tokens = Arc::new(AtomicUsize::new(max_tokens));
    let result_chunks = Arc::new(Mutex::new(Vec::with_capacity(total_files)));

    // Step 3: Parallel processing of files
    {
      profile!("Processing and truncating diffs");
      let model = Arc::new(model);

      // Process files in parallel chunks
      const CHUNK_SIZE: usize = 10;
      let chunks: Vec<_> = files
        .into_iter() // Convert to owned chunks
        .collect::<Vec<_>>()
        .chunks(CHUNK_SIZE)
        .map(|chunk| chunk.to_vec())
        .collect();

      // Process chunks in parallel
      let processing_result: Result<()> = thread_pool.install(|| {
        chunks.par_iter().try_for_each(|chunk| {
          // Pre-compute token counts for the chunk
          let token_counts: Vec<_> = chunk
            .par_iter()
            .map(|(path, content)| {
              let model = Arc::clone(&model);
              let count = model.count_tokens(content).unwrap_or_default();
              (path.clone(), count)
            })
            .collect();

          // Process files in the chunk
          let mut chunk_results = Vec::with_capacity(chunk.len());
          for (idx, ((path, content), (_, token_count))) in chunk.iter().zip(token_counts.iter()).enumerate() {
            // Calculate token budget atomically
            let total_remaining = remaining_tokens.load(Ordering::Relaxed);
            let files_remaining = total_files.saturating_sub(idx);
            let max_tokens_per_file = total_remaining.saturating_div(files_remaining);

            if max_tokens_per_file == 0 {
              continue; // Skip this file if no tokens left
            }

            let token_count = *token_count;
            let allocated_tokens = token_count.min(max_tokens_per_file);

            // Try to claim tokens atomically
            let old_remaining = remaining_tokens.fetch_sub(allocated_tokens, Ordering::Relaxed);
            if old_remaining < allocated_tokens {
              // Restore tokens if we couldn't claim them
              remaining_tokens.fetch_add(allocated_tokens, Ordering::Relaxed);
              continue;
            }

            // Process the file with allocated tokens
            let processed_content = if token_count > allocated_tokens {
              model.truncate(content, allocated_tokens)?
            } else {
              content.clone()
            };

            chunk_results.push((path.clone(), processed_content));
          }

          // Store results in order
          if !chunk_results.is_empty() {
            result_chunks.lock().extend(chunk_results);
          }
          Ok(())
        })
      });

      // Handle any processing errors
      processing_result?;
    }

    // Combine results in order
    let results = result_chunks.lock();
    let mut final_result = String::with_capacity(results.iter().map(|(_, content)| content.len()).sum());

    for (_, content) in results.iter() {
      if !final_result.is_empty() {
        final_result.push('\n');
      }
      final_result.push_str(content);
    }

    Ok(final_result)
  }
}

pub trait PatchRepository {
  fn to_patch(&self, tree: Option<Tree<'_>>, max_token_count: usize, model: Model) -> Result<String>;
  fn to_diff(&self, tree: Option<Tree<'_>>) -> Result<git2::Diff<'_>>;
}

impl PatchRepository for Repository {
  fn to_patch(&self, tree: Option<Tree>, max_token_count: usize, model: Model) -> Result<String> {
    profile!("Repository patch generation");
    // Generate diff and process it
    let diff = self.to_diff(tree)?;
    diff.to_patch(max_token_count, model)
  }

  fn to_diff(&self, tree: Option<Tree<'_>>) -> Result<git2::Diff<'_>> {
    profile!("Git diff generation");
    let mut opts = DiffOptions::new();
    opts
      .ignore_whitespace_change(true)
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

    self
      .diff_tree_to_index(tree.as_ref(), None, Some(&mut opts))
      .context("Failed to get diff")
  }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "commit-msg-hook", about = "A tool for generating commit messages.")]
pub struct Args {
  pub commit_msg_file: PathBuf,

  #[structopt(short = "t", long = "type")]
  pub commit_type: Option<String>,

  #[structopt(short = "s", long = "sha1")]
  pub sha1: Option<String>
}

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

  // anyhow
  #[error(transparent)]
  Anyhow(#[from] anyhow::Error)
}
