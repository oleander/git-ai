use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::fs::File;

use git2::{Diff, DiffFormat, DiffOptions, Repository, Tree};
use anyhow::{Context, Result};
use thiserror::Error;
use clap::Parser;
use tokio::io::AsyncReadExt;

use crate::commit::ChatError;

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
    self.new_file().path().or_else(|| self.old_file().path()).map(PathBuf::from).unwrap_or_default()
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
  fn to_patch(&self, max_token_count: usize) -> Result<String>;
}

impl PatchDiff for Diff<'_> {
  fn to_patch(&self, max_token_count: usize) -> Result<String> {
    let truncated_message = "<truncated>";
    let number_of_files = self.deltas().len();

    if number_of_files == 0 {
      return Err(HookError::EmptyDiffOutput.into());
    }

    let tokens_per_file = (max_token_count / number_of_files) - truncated_message.len();
    let mut token_table: HashMap<PathBuf, usize> = HashMap::new();
    let mut patch_acc = Vec::new();

    for delta in self.deltas() {
      token_table.insert(delta.path(), 0);
    }

    #[rustfmt::skip]
    self.print(DiffFormat::Patch, |diff, _hunk, line| {
      let diff_path = diff.path();
      let Some(tokens) = token_table.get_mut(&diff_path) else {
        return true;
      };

      let content = line.content();
      if *tokens + content.len() <= tokens_per_file {
        patch_acc.extend_from_slice(content);
        *tokens += content.to_utf8().split_whitespace().count();
      } else {
        patch_acc.extend_from_slice(truncated_message.as_bytes());
        token_table.remove(&diff_path);
      }

      true
    }).context("Failed to print diff")?;

    Ok(patch_acc.to_utf8())
  }
}

pub trait PatchRepository {
  fn to_patch(&self, tree: Option<Tree<'_>>, max_token_count: usize) -> Result<String>;
  fn to_diff(&self, tree: Option<Tree<'_>>) -> Result<git2::Diff<'_>>;
}

impl<'a> PatchRepository for Repository {
  fn to_patch(&self, tree: Option<Tree>, max_token_count: usize) -> Result<String> {
    self.to_diff(tree)?.to_patch(max_token_count)
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

    self.diff_tree_to_index(tree.as_ref(), None, Some(&mut opts)).context("Failed to get diff")
  }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
  pub commit_msg_file: PathBuf,

  #[clap(required = false)]
  pub commit_type: Option<String>,

  #[clap(required = false)]
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
  Anyhow(#[from] anyhow::Error),

  // ChatError
  #[error(transparent)]
  Chat(#[from] ChatError)
}
