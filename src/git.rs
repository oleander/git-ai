#![allow(dead_code)]
#![allow(unused_imports)]

use git2::{
  Commit, Delta, Diff, DiffFormat, DiffOptions, Index, IndexAddOption, ObjectType, Oid, Repository, RepositoryInitOptions, RepositoryOpenFlags as Flag, StatusOptions, StatusShow
};
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use anyhow::{anyhow, bail, Context, Result};
use log::{debug, error, info, trace, warn};
use std::collections::HashSet;
use lazy_static::lazy_static;
use std::process::Command;
use std::path::Path;
use crate::chat;

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
    let repo = Repository::open_ext(path, Flag::empty(), Vec::<&Path>::new())
      .with_context(|| format!("Failed to open the git repository at"))?;

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
        let tree = head
          .resolve()
          .context("Failed to resolve head")?
          .peel(ObjectType::Commit)
          .context("Failed to peel head")?
          .into_commit()
          .map_err(|_| anyhow!("Failed to resolve commit"))?
          .tree()
          .context("Failed to get tree")?;
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
      bail!("No files to commit");
    }

    /* Will abort if the diff is too long */
    diff
      .print(git2::DiffFormat::Patch, |_, _, line| {
        let content = line.content();
        diff_str.extend_from_slice(content);
        let str = content.to_utf8();
        length += str.len();
        length <= max_token_count
      })
      .ok();

    let mut diff_output = diff_str.to_utf8();
    if diff_output.is_empty() {
      bail!("Empty diff output");
    }

    /* If the diff output is too long, truncate it */
    if diff_output.len() > max_token_count {
      diff_output.truncate(max_token_count);
    }

    debug!("[diff] Diff: {}", diff_output);

    Ok((diff_output, files))
  }

  pub async fn commit(&self, add_all: bool) -> Result<()> {
    debug!("[commit] Committing with message");

    let repo = self.repo.read().expect("Failed to lock repo");
    let mut index = repo.index().expect("Failed to get index");

    if add_all {
      debug!("Adding all files to index(--all)");

      index.add_all(["*"], IndexAddOption::DEFAULT, None).context("Failed to add all files to index")?;
      index.write().context("Failed to write index")?;
    }

    let (diff, _) = self.diff(1000)?;
    let oid = index.write_tree().context("Failed to write tree")?;
    let tree = repo.find_tree(oid).context("Failed to find tree")?;
    let signature = repo.signature().context("Failed to get signature")?;
    let message = chat::suggested_commit_message(diff).await?;

    match repo.head() {
      Ok(ref head) => {
        let parent = head
          .resolve()
          .context("Failed to resolve head")?
          .peel(ObjectType::Commit)
          .context("Failed to peel head")?
          .into_commit()
          .map_err(|_| anyhow!("Failed to resolve parent commit"))?;

        repo
          .commit(Some("HEAD"), &signature, &signature, &message, &tree, &[&parent])
          .context("Failed to commit (1)")?;
      },
      Err(_) => {
        repo.commit(Some("HEAD"), &signature, &signature, &message, &tree, &[]).context("Failed to commit (2)")?;
      }
    }

    Ok(())
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
