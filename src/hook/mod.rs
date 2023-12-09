// Hook: prepare-commit-msg

use std::io::{Read, Write};
use std::time::Duration;
use std::path::PathBuf;
use std::fs::File;

#[cfg(not(mock))]
use git2::{DiffFormat, DiffOptions, Oid, Repository, Tree};
use indicatif::{ProgressBar, ProgressStyle};
use anyhow::{bail, Context, Result};
use lazy_static::lazy_static;
use dotenv_codegen::dotenv;
use clap::Parser;

use crate::chat::generate_commit;
use crate::chat::ChatError;
use crate::config;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
  pub commit_msg_file: PathBuf,

  #[clap(required = false)]
  pub commit_type: Option<String>,

  #[clap(required = false)]
  pub sha1: Option<Oid>
}

use thiserror::Error;

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

lazy_static! {
  static ref MAX_DIFF_TOKENS: usize = dotenv!("MAX_DIFF_TOKENS").parse::<usize>().unwrap();
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
    let mut length = 0;

    #[rustfmt::skip]
    self.print(DiffFormat::Patch, |_, _, line| {
      let content = line.content();
      acc.extend_from_slice(content);
      let str = content.to_utf8();
      length += str.len();
      length <= max_token_count
    }).ok();

    Ok(acc.to_utf8())
  }
}

pub trait PatchRepository {
  fn to_patch(&self, tree: Option<Tree<'_>>, max_token_count: usize) -> Result<String>;
}

impl PatchRepository for Repository {
  fn to_patch(&self, tree: Option<Tree<'_>>, max_token_count: usize) -> Result<String> {
    let mut opts = DiffOptions::new();
    opts
      .enable_fast_untracked_dirs(true)
      .ignore_whitespace_change(true)
      .recurse_untracked_dirs(false)
      .recurse_ignored_dirs(false)
      .ignore_whitespace_eol(true)
      .ignore_blank_lines(true)
      .include_untracked(false)
      .indent_heuristic(false)
      .ignore_submodules(true)
      .include_ignored(false)
      .interhunk_lines(0)
      .context_lines(0)
      .patience(true)
      .minimal(true);

    self.diff_tree_to_index(tree.as_ref(), None, Some(&mut opts))?.to_patch(max_token_count)
  }
}

#[cfg(mock)]
async fn generate_commit_message(diff: String) -> Result<String> {
  Ok(diff.to_string())
}

pub async fn run(args: &Args) -> Result<(), HookError> {
  // If defined, then the user already provided a commit message
  if args.commit_type.is_some() {
    return Ok(());
  }

  // Loading bar to indicate that the program is running
  let style = ProgressStyle::default_spinner()
    .tick_strings(&["-", "\\", "|", "/"])
    .template("{spinner:.blue} {msg}")
    .context("Failed to create progress bar style")?;

  let pb = ProgressBar::new_spinner();
  pb.set_style(style);
  pb.set_message("Generating commit message...");
  pb.enable_steady_tick(Duration::from_millis(150));

  let repo = Repository::open_from_env().context("Failed to open repository")?;

  // Get the tree from the commit if the sha1 is provided
  // The sha1 is provided when the user is amending a commit
  let tree = if let Some(sha1) = args.sha1 {
    repo.find_commit(sha1).ok().and_then(|commit| commit.tree().ok())
  } else {
    repo.head().ok().and_then(|head| head.peel_to_tree().ok())
  };

  let max_tokens = config::get("max-diff-tokens").unwrap_or(*MAX_DIFF_TOKENS);
  let patch = repo.to_patch(tree, max_tokens).context("Failed to get patch")?;

  if patch.is_empty() {
    Err(HookError::EmptyDiffOutput)?;
  }

  let commit_message = generate_commit(patch.to_string()).await?;

  args
    .commit_msg_file
    .write(commit_message.trim().to_string())
    .context("Failed to write commit message")?;

  pb.finish_and_clear();
  Ok(())
}
