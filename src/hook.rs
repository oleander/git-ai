use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::fs::File;
use std::sync::Arc;

use structopt::StructOpt;
use git2::{Diff, DiffFormat, DiffOptions, Repository, Tree};
use anyhow::{bail, Context, Result};
use thiserror::Error;
use rayon::prelude::*;

use crate::model::Model;
use crate::profile;

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
}

impl PatchDiff for Diff<'_> {
  fn to_patch(&self, max_tokens: usize, model: Model) -> Result<String> {
    profile!("Generating patch diff");
    let mut files: HashMap<PathBuf, String> = HashMap::new();

    {
      profile!("Processing diff changes");
      self.print(DiffFormat::Patch, |diff, _hunk, line| {
        let content = line.content().to_utf8();
        let line_content = match line.origin() {
          '+' | '-' => content,
          _ => format!("context: {}", content)
        };

        files
          .entry(diff.path())
          .or_insert_with(|| String::with_capacity(4096))
          .push_str(&line_content);
        true
      })?;
    }

    let mut result = String::with_capacity(files.values().map(|s| s.len()).sum());
    let mut remaining_tokens = max_tokens;
    let total_files = files.len();

    {
      profile!("Processing and truncating diffs");

      // Convert model to Arc for thread-safe sharing
      let model = Arc::new(model);

      // Pre-compute token counts in parallel
      let file_tokens: HashMap<PathBuf, usize> = files
        .iter()
        .collect::<Vec<_>>()
        .par_iter()
        .map(|(path, content)| {
          let model = Arc::clone(&model);
          let count = model.count_tokens(content).unwrap_or_default();
          ((*path).clone(), count)
        })
        .collect();

      // Process files sequentially since we need to maintain token budget
      for (index, (path, diff)) in files.iter().enumerate() {
        let files_remaining = total_files.saturating_sub(index);
        let max_tokens_per_file = remaining_tokens.saturating_div(files_remaining);

        if max_tokens_per_file == 0 {
          bail!("No tokens left to generate commit message. Try increasing the max-tokens configuration option using `git ai config set max-tokens <value>`");
        }

        let file_token_count = file_tokens.get(path).copied().unwrap_or_default();
        let file_allocated_tokens = file_token_count.min(max_tokens_per_file);

        let truncated_content = if file_token_count > file_allocated_tokens {
          model.truncate(diff, file_allocated_tokens)?
        } else {
          diff.clone()
        };

        if !result.is_empty() {
          result.push('\n');
        }
        result.push_str(&truncated_content);
        remaining_tokens = remaining_tokens.saturating_sub(file_allocated_tokens);
      }
    }

    Ok(result)
  }
}

pub trait PatchRepository {
  fn to_patch(&self, tree: Option<Tree<'_>>, max_token_count: usize, model: Model) -> Result<String>;
  fn to_diff(&self, tree: Option<Tree<'_>>) -> Result<git2::Diff<'_>>;
}

impl PatchRepository for Repository {
  fn to_patch(&self, tree: Option<Tree>, max_token_count: usize, model: Model) -> Result<String> {
    profile!("Repository patch generation");
    self.to_diff(tree)?.to_patch(max_token_count, model)
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
