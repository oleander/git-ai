// Hook: prepare-commit-msg
mod traits;

use std::path::PathBuf;

#[cfg(not(mock))]
use git2::{Oid, Repository};
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use dotenv_codegen::dotenv;
use clap::Parser;

use crate::chat::{generate_commit, ChatError};
use crate::hook::traits::{FilePath, PatchRepository};
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



#[cfg(mock)]
async fn generate_commit_message(diff: String) -> Result<String> {
  Ok(diff.to_string())
}

pub async fn run(args: &Args) -> Result<(), HookError> {
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

  Ok(())
}
