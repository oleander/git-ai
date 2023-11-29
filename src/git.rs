use std::sync::{PoisonError, RwLockReadGuard};
use git2::{RepositoryOpenFlags as Flag, *};
use log::{debug, error, warn};
use crate::chat::ChatError;
use thiserror::Error;
use anyhow::Context;
use std::path::Path;

#[derive(Error, Debug)]
pub enum GitError {
  #[error("Git error: {0}")]
  Git(#[from] git2::Error),

  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),

  #[error("No files to commit")]
  NoFilesToCommit,

  #[error("Empty diff output")]
  EmptyDiffOutput,

  #[error("Anyhow error: {0}")]
  Anyhow(#[from] anyhow::Error),

  #[error("Chat error: {0}")]
  ChatError(#[from] ChatError)
}

pub type Result<T, E = GitError> = std::result::Result<T, E>;

impl From<PoisonError<RwLockReadGuard<'_, git2::Repository>>> for GitError {
  fn from(_: PoisonError<RwLockReadGuard<'_, git2::Repository>>) -> Self {
    GitError::Git(git2::Error::from_str("Failed to lock repo"))
  }
}

impl From<git2::Object<'_>> for GitError {
  fn from(_: git2::Object<'_>) -> Self {
    GitError::Git(git2::Error::from_str("Failed to get git2 object"))
  }
}

pub struct Repo {
  repo: Repository
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
    Ok(Repo {
      repo: Repository::open_ext(path, Flag::empty(), Vec::<&Path>::new())?
    })
  }

  pub fn diff(&self, max_token_count: usize) -> Result<(String, Vec<String>)> {
    let mut files = Vec::new();
    let mut diff_str = Vec::new();
    let mut opts = Repo::diff_options();
    let mut length = 0;

    let tree = self.repo.head().ok().and_then(|head| head.peel_to_tree().ok());
    let diff = self.repo.diff_tree_to_workdir_with_index(tree.as_ref(), Some(&mut opts))?;

    debug!("Stats: {:?}", diff.stats());
    debug!("Tree: {:?}", tree);

    diff.foreach(
      &mut |delta, _| {
        if let Some(file) = delta.new_file().path() {
          let file_path = file.to_string_lossy().into_owned();
          debug!("[diff] File: {}", file_path);
          files.push(file_path);
        } else {
          warn!("[diff] Could not get file path");
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

  pub fn add_all(&self) -> Result<()> {
    debug!("[add_all] Adding all files to index(--all)");

    let mut index = self.repo.index().expect("Failed to get index");
    index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
    index.write().context("Could not write index")?;
    Ok(())
  }

  pub fn commit(&self, message: &str) -> Result<Oid> {
    debug!("[commit] Committing with message");

    let mut index = self.repo.index().expect("Failed to get index");
    let oid = index.write_tree().context("Could not write tree")?;
    let tree = self.repo.find_tree(oid).context("Could not find tree")?;
    let signature = self.repo.signature().context("Could not get signature")?;
    let parent = self.repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents = parent.iter().collect::<Vec<&Commit>>();

    self
      .repo
      .commit(Some("HEAD"), &signature, &signature, &message, &tree, parents.as_slice())
      .context("Could not commit")
      .map_err(GitError::from)
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
