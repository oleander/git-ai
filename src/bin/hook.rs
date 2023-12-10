// Hook: prepare-commit-msg

use std::path::PathBuf;

#[cfg(not(mock))]
use git2::{Oid, Repository};
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use dotenv_codegen::dotenv;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use thiserror::Error;


use ai::hook::traits::*;
use ai::chat::{generate_commit, ChatError};
use ai::hook::traits::{FilePath, PatchRepository};
use ai::config;

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();
  let args = Args::parse();

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

  Ok(())
}
