#![allow(dead_code)]
#![allow(unused_imports)]

use git2::{
  Commit, Delta, DiffFormat, DiffOptions, Index, IndexAddOption, ObjectType, Oid, Repository, RepositoryInitOptions, RepositoryOpenFlags as Flag, StatusOptions, StatusShow
};
use anyhow::{anyhow, bail, Context, Result};
use log::{debug, error, info, trace, warn};
use std::sync::{LazyLock, Mutex};
use lazy_static::lazy_static;
use std::process::Command;
use std::path::Path;
use crate::chat;

lazy_static! {
  pub static ref REPO: Repo =
    Repo::new().expect("Failed to initialize git repository");
}

pub struct Repo {
  repo: Mutex<Repository>
}

impl Repo {
  pub fn new() -> Result<Self> {
    let repo = Repository::open_ext(".", Flag::empty(), Vec::<&Path>::new())
      .with_context(|| format!("Failed to open the git repository at"))?;

    Ok(Repo {
      repo: Mutex::new(repo)
    })
  }

  pub fn opts(&self) -> DiffOptions {
    let mut opts = DiffOptions::new();
    opts
      .enable_fast_untracked_dirs(true)
      .ignore_whitespace_change(true)
      .recurse_untracked_dirs(false)
      .recurse_ignored_dirs(false)
      .ignore_whitespace_eol(true)
      .recurse_untracked_dirs(false)
      .ignore_blank_lines(true)
      .ignore_submodules(true)
      .include_untracked(false)
      .include_ignored(false)
      .interhunk_lines(0)
      .context_lines(0)
      .minimal(true);
    opts
  }

  pub fn diff(
    &self, max_token_count: usize, repo: &Repository, index: Index
  ) -> Result<(String, Index)> {
    debug!("[diff] Generating diff with max token count: {}", max_token_count);

    let mut buf = Vec::new();
    let mut opts = self.opts();
    let mut count = 0;
    let tree = repo.head().context("Failed to get head")?.peel_to_tree()?;
    let diff = repo
      .diff_tree_to_index(Some(&tree), Some(&index), Some(&mut opts))
      .context("Failed to diff tree to index")?;

    diff
      .foreach(
        &mut |_file, _progress| true,
        None,
        None,
        Some(&mut |_delta, _hunk, line| {
          let content = line.content();
          let tokens: Vec<&[u8]> =
            content.split(|c| c.is_ascii_whitespace()).collect();
          let new_count = count + tokens.len();

          if new_count > max_token_count {
            return false;
          }

          buf.extend_from_slice(content);
          count = new_count;
          true
        })
      )
      .context("Failed to iterate over diff")?;

    if buf.is_empty() {
      bail!("Nothing to commit");
    }

    let str =
      String::from_utf8(buf).context("Failed to convert diff to string")?;
    Ok((str, index))
  }

  pub async fn commit(&self, add_all: bool) -> Result<()> {
    debug!("[commit] Committing with message");

    let repo = self.repo.lock().expect("Failed to lock repo");
    let mut index = repo.index().expect("Failed to get index");

    if add_all {
      debug!("Adding all files to index(--all)");

      index
        .add_all(["*"], IndexAddOption::DEFAULT, None)
        .context("Failed to add all files to index")?;
      index.write().context("Failed to write index")?;
    }

    let (diff, mut index) =
      self.diff(1000, &repo, index).context("Failed to generate diff")?;
    let oid = index.write_tree().context("Failed to write tree")?;
    let tree = repo.find_tree(oid).context("Failed to find tree")?;
    let signature = repo.signature().context("Failed to get signature")?;
    let parent = repo
      .head()
      .context("Failed to get head (2)")?
      .resolve()
      .context("Failed to resolve head")?
      .peel(ObjectType::Commit)
      .context("Failed to peel head")?
      .into_commit()
      .map_err(|_| anyhow!("Failed to resolve parent commit"))?;

    let message = chat::suggested_commit_message(diff)
      .await
      .context("Failed to generate commit message")?;

    repo
      .commit(Some("HEAD"), &signature, &signature, &message, &tree, &[&parent])
      .context("Failed to commit")?;

    Ok(())
  }
}

pub fn repo() -> &'static Repo {
  &REPO
}
