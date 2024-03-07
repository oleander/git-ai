// Hook: prepare-commit-msg

use std::time::Duration;

use git2::Repository;
use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use ai::hook::{FilePath, PatchRepository, *};
use ai::{commit, config};

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();

  // Show cursor on exit whenever ctrl-c is pressed
  ctrlc::set_handler(move || {
    console::Term::stdout().show_cursor().expect("Failed to show cursor");
  })?;

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
  let tree = match args.sha1.as_ref() {
    Some("HEAD") => repo.head().ok().and_then(|head| head.peel_to_tree().ok()),
    Some(sha1) => repo.find_commit(sha1.parse()?).ok().and_then(|commit| commit.tree().ok()),
    None => repo.head().ok().and_then(|head| head.peel_to_tree().ok())
  };

  let max_tokens = config::APP.max_diff_tokens;
  let patch = repo.to_patch(tree, max_tokens).context("Failed to get patch")?;

  if patch.is_empty() {
    Err(HookError::EmptyDiffOutput)?;
  }

  let commit_message = commit::generate(patch.to_string()).await?;

  args
    .commit_msg_file
    .write(commit_message.trim().to_string())
    .context("Failed to write commit message")?;

  Ok(())
}
