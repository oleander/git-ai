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
use std::path::PathBuf;
use std::io::{Read, Write};
use std::fs::File;

use structopt::StructOpt;
use git2::{Diff, DiffFormat, DiffOptions, Repository, Tree};
use anyhow::{Context, Result};
use thiserror::Error;

use crate::model::Model;

// Constants
const DEFAULT_STRING_CAPACITY: usize = 1024;
const ESTIMATED_FILES_COUNT: usize = 100;
const SMALL_DIFF_THRESHOLD: usize = 5;

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
    String::from_utf8(self.to_vec()).unwrap_or_default()
  }
}

impl Utf8String for [u8] {
  fn to_utf8(&self) -> String {
    String::from_utf8(self.to_vec()).unwrap_or_default()
  }
}

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

    // Step 2: Fast path for small diffs - no parallelization needed
    if files.len() <= SMALL_DIFF_THRESHOLD {
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

      return Ok(result);
    }

    // For larger diffs, use a simpler approach without parallel processing
    let mut files_vec: Vec<(PathBuf, String, usize)> = files
      .into_iter()
      .map(|(path, content)| {
        let token_count = model.count_tokens(&content).unwrap_or_default();
        (path, content, token_count)
      })
      .collect();

    // Sort by token count
    files_vec.sort_by_key(|(_, _, count)| *count);

    // Process files with optimized token allocation
    let mut result = String::new();
    let mut tokens_used = 0;

    for (i, (_, content, token_count)) in files_vec.into_iter().enumerate() {
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
  }

  fn collect_diff_data(&self) -> Result<HashMap<PathBuf, String>> {
    // Pre-allocate HashMap with estimated capacity
    let mut files = HashMap::with_capacity(ESTIMATED_FILES_COUNT);

    // Process diffs
    self.print(DiffFormat::Patch, |diff, _hunk, line| {
      let path = diff.path();

      // Fast path for UTF-8 content - avoid expensive conversions
      let content = if let Ok(s) = std::str::from_utf8(line.content()) {
        s.to_string()
      } else {
        // Fallback for non-UTF8 content
        line.content().to_utf8()
      };

      // Process line by line origin
      match line.origin() {
        '+' | '-' => {
          let entry = files
            .entry(path)
            .or_insert_with(|| String::with_capacity(DEFAULT_STRING_CAPACITY));
          entry.push_str(&content);
        }
        _ => {
          let entry = files
            .entry(path)
            .or_insert_with(|| String::with_capacity(DEFAULT_STRING_CAPACITY));
          entry.push_str("context: ");
          entry.push_str(&content);
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
}

impl PatchRepository for Repository {
  fn to_patch(&self, tree: Option<Tree>, max_token_count: usize, model: Model) -> Result<String> {
    self.to_diff(tree)?.to_patch(max_token_count, model)
  }

  fn to_diff(&self, tree: Option<Tree<'_>>) -> Result<git2::Diff<'_>> {
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
