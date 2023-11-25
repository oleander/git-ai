#![allow(dead_code)]
#![allow(unused_imports)]

use git2::{
  Commit, Delta, Diff, DiffFormat, DiffOptions, Index, IndexAddOption, ObjectType, Oid, Repository, RepositoryInitOptions, RepositoryOpenFlags as Flag, StatusOptions, StatusShow
};
use std::sync::{Arc, LazyLock, Mutex, PoisonError, RwLock, RwLockReadGuard};
// use anyhow::{anyhow, bail, Context, Result};
use log::{debug, error, info, trace, warn};
use std::collections::HashSet;
use lazy_static::lazy_static;
use std::process::Command;
use std::path::Path;
use crate::chat::ChatError;
use thiserror::Error;
use anyhow::bail;
use crate::chat;

#[derive(Error, Debug)]
pub enum GitError {
  #[error("Git error: {0}")]
  Git(#[from] git2::Error),
  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),
  #[error("No files to commit")]
  NoFilesToCommit,
  #[error("Empty diff output")]
  EmptyDiffOutput // Add other error types as needed here
}

impl From<PoisonError<RwLockReadGuard<'_, git2::Repository>>> for GitError {
  fn from(_: PoisonError<RwLockReadGuard<'_, git2::Repository>>) -> Self {
    GitError::Git(git2::Error::from_str("Failed to lock repo"))
  }
}

impl From<git2::Object<'_>> for GitError {
  fn from(_: git2::Object<'_>) -> Self {
    GitError::Git(git2::Error::from_str("Failed to get object"))
  }
}

impl From<ChatError> for GitError {
  fn from(_: ChatError) -> Self {
    GitError::Git(git2::Error::from_str("Failed to get object"))
  }
}

pub type Result<T, E = GitError> = std::result::Result<T, E>;

pub struct Repo {
  repo: Arc<RwLock<Repository>>
}

trait Utf8String {
  fn to_utf8(&self) -> String;
}

impl Utf8String for [u8] {
  fn to_utf8(&self) -> String {
    String::from_utf8(self.to_vec()).unwrap_or_default()
  }
}

impl Repo {
  pub fn new() -> Result<Self> {
    Self::new_with_path(".".to_string())
  }

  pub fn new_with_path(path: String) -> Result<Self> {
    let repo = Repository::open_ext(path, Flag::empty(), Vec::<&Path>::new())?;

    Ok(Repo {
      repo: Arc::new(RwLock::new(repo))
    })
  }

  pub fn diff(&self, max_token_count: usize) -> Result<(String, Vec<String>)> {
    let repo = self.repo.read().expect("Failed to lock repo");
    let mut files = Vec::new();
    let mut diff_str = Vec::new();
    let mut opts = Repo::diff_options();
    let mut length = 0;

    let diff = match repo.head() {
      Ok(ref head) => {
        let tree = head.resolve()?.peel(ObjectType::Commit)?.into_commit()?.tree().map_err(GitError::from)?;
        repo.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts))?
      },
      Err(_) => repo.diff_tree_to_workdir_with_index(None, Some(&mut opts))?
    };

    diff.foreach(
      &mut |delta, _| {
        if let Some(file) = delta.new_file().path() {
          let file_path = file.to_string_lossy().into_owned();
          files.push(file_path);
        }
        true
      },
      None,
      None,
      None
    )?;

    if files.is_empty() {
      return Err(GitError::NoFilesToCommit);
    }

    /* Will abort if the diff is too long */
    diff
      .print(DiffFormat::Patch, |_, _, line| {
        let content = line.content();
        diff_str.extend_from_slice(content);
        let str = content.to_utf8();
        length += str.len();
        length <= max_token_count
      })
      .ok();

    let mut diff_output = diff_str.to_utf8();
    if diff_output.is_empty() {
      return Err(GitError::EmptyDiffOutput);
    }

    /* If the diff output is too long, truncate it */
    if diff_output.len() > max_token_count {
      diff_output.truncate(max_token_count);
    }

    debug!("[diff] Diff: {}", diff_output);

    Ok((diff_output, files))
  }

  pub async fn commit(&self, add_all: bool) -> Result<Oid> {
    debug!("[commit] Committing with message");

    let repo = self.repo.read().expect("Failed to lock repo");
    let mut index = repo.index().expect("Failed to get index");

    if add_all {
      debug!("Adding all files to index(--all)");

      index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
      index.write()?
    }

    let (diff, _) = self.diff(1000)?;
    let oid = index.write_tree()?;
    let tree = repo.find_tree(oid)?;
    let signature = repo.signature()?;
    let message = chat::suggested_commit_message(diff).await?;


    let parent = match repo.head() {
      Ok(head) => head.resolve()?.peel(ObjectType::Commit)?.into_commit().map(Some)?,
      Err(_) => None
    };

    match parent {
      Some(parent) => {
        repo.commit(Some("HEAD"), &signature, &signature, &message, &tree, &[&parent])
      },
      None => {
        repo.commit(Some("HEAD"), &signature, &signature, &message, &tree, &[])
      }
    }
  }

  fn diff_options() -> DiffOptions {
    let mut opts = DiffOptions::new();
    opts
      .enable_fast_untracked_dirs(true)
      .ignore_whitespace_change(true)
      .recurse_untracked_dirs(false)
      .recurse_ignored_dirs(false)
      .ignore_whitespace_eol(true)
      .ignore_blank_lines(true)
      .ignore_submodules(true)
      .include_untracked(false)
      .include_ignored(false)
      .interhunk_lines(0)
      .context_lines(0)
      .minimal(true)
      .patience(true)
      .indent_heuristic(false);
    opts
  }
}
