// Hook: prepare-commit-msg

use indicatif::{ProgressBar, ProgressStyle};
use anyhow::{Context, Result};
use git2::{Oid, Repository};
use ai::{commit, config};
use std::time::Duration;
use ai::commit::Session;
use termion::event::Key;
use clap::Parser;
use ai::hook::*;

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();

  let args = Args::parse();
  let max_tokens = config::APP.max_diff_tokens;
  let pb = ProgressBar::new_spinner();
  let repo = Repository::open_from_env().context("Failed to open repository")?;

  // If defined, then the user already provided a commit message
  if args.commit_type.is_some() {
    return Ok(());
  }

  // Loading bar to indicate that the program is running
  let style = ProgressStyle::default_spinner()
    .tick_strings(&["-", "\\", "|", "/"])
    .template("{spinner:.blue} {msg}")
    .context("Failed to create progress bar style")?;

  pb.set_style(style);
  pb.set_message("Generating commit message...");
  pb.enable_steady_tick(Duration::from_millis(150));

  let tree = match args.sha1.as_deref() {
    // git commit --amend or git commit -c
    Some("HEAD") | None => repo.head().ok().and_then(|head| head.peel_to_tree().ok()),
    // git rebase
    Some(sha1) => repo.find_object(Oid::from_str(sha1)?, None).ok().and_then(|obj| obj.peel_to_tree().ok())
  };

  let patch = repo.to_patch(tree, max_tokens).context("Failed to get patch")?;

  if patch.is_empty() {
    Err(HookError::EmptyDiffOutput)?;
  }

  let pb_clone = pb.clone();
  ctrlc::set_handler(move || {
    pb_clone.finish_and_clear();
    console::Term::stdout().show_cursor().expect("Failed to show cursor");
    std::process::exit(1);
  })?;

  // Create a new session from the client
  let session = Session::load_from_repo(&repo).await.unwrap();

  // If the user has a session, then we can use it to generate the commit message
  let response = commit::generate(patch.to_string(), session.into(), pb.clone().into()).await?;

  // Write the response to the commit message file
  args.commit_msg_file.write(response.response.trim().to_string()).unwrap();

  // Save the session to the repository
  response.session.save_to_repo(&repo).await.unwrap();

  pb.finish_and_clear();

  Ok(())
}
