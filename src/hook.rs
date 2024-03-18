use std::fmt::{self, Display, Formatter};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::fs::File;

use git2::{Delta, DiffFormat, DiffOptions, Repository, Tree};
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use dotenv_codegen::dotenv;
use thiserror::Error;
use clap::Parser;

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

impl PatchDiff for git2::Diff<'_> {
  fn to_patch(&self, max_token_count: usize) -> Result<String> {
    let mut acc = Vec::new();
    let length = 0;

    #[rustfmt::skip]
    self.print(DiffFormat::Patch, |_, _, line| {
      let content = line.content();
      acc.extend_from_slice(content);
      length <= max_token_count
    }).ok();

    Ok(acc.to_utf8())
  }
}

pub trait PatchRepository {
  fn to_patch(&self, tree: Option<Tree<'_>>, max_token_count: usize) -> Result<String>;
  fn to_diff(&self, tree: Option<Tree<'_>>) -> Result<git2::Diff<'_>>;
}

#[derive(Debug, Error)]
enum PatchError {
  #[error("Error accessing repository: {0}")]
  RepositoryAccessError(String),
  #[error("Error calculating diff: {0}")]
  DiffCalculationError(String)
}

#[derive(Debug, Clone)]
enum DeltaStatus {
  Added(PathBuf),
  Modified(PathBuf),
  Deleted(PathBuf),
  Renamed(PathBuf, PathBuf),
  Ignored
}

impl DeltaStatus {
  fn from(delta: &git2::DiffDelta) -> Result<DeltaStatus> {
    let path = delta.new_file().path().or(delta.old_file().path()).ok_or_else(|| {
      PatchError::DiffCalculationError("Failed to retrieve path for delta".to_string())
    })?;

    let owned_path = path.to_path_buf();

    let r = match delta.status() {
      Delta::Added => DeltaStatus::Added(owned_path),
      Delta::Modified => DeltaStatus::Modified(owned_path),
      Delta::Deleted => DeltaStatus::Deleted(owned_path),
      _ => DeltaStatus::Ignored
    };

    Ok(r)
  }
}

impl Display for DeltaStatus {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      DeltaStatus::Added(path) => {
        write!(f, "A {}", path.to_string_lossy())?;
        Ok(())
      },
      DeltaStatus::Modified(path) => {
        write!(f, "M {}", path.to_string_lossy())?;
        Ok(())
      },
      DeltaStatus::Deleted(path) => {
        write!(f, "D {}", path.to_string_lossy())?;
        Ok(())
      },
      DeltaStatus::Renamed(old, new) => {
        write!(f, "R {} {}", old.to_string_lossy(), new.to_string_lossy())?;
        Ok(())
      },
      DeltaStatus::Ignored => {
        write!(f, "")?;
        Ok(())
      }
    }
  }
}

#[derive(Debug)]
struct PatchSummary(Vec<DeltaStatus>);

impl PatchSummary {
  fn to_patch(&self, max_token_count: usize) -> Result<String> {
    let tokens_per_delta = max_token_count / self.0.len();
    let res = self.0.iter().collect::<Vec<_>>();
    let lines: Vec<_> = res
      .iter()
      .map(|delta| delta.to_string().chars().take(tokens_per_delta).collect::<String>())
      .collect();
    let r = Ok(lines.join("\n"));
    println!("{:?}", r);
    r
  }
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

    self
      .diff_tree_to_index(tree.as_ref(), None, Some(&mut opts))
      .context("Failed to get diff")
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
