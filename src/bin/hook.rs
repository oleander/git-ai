// Hook: prepare-commit-msg

use std::io::{stdin, stdout};
use tokio::{io, sync::mpsc};
use std::time::Duration;

use git2::Repository;
use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use ai::hook::*;
use termion::raw::IntoRawMode;
use termion::input::TermRead;
use indicatif_log_bridge::LogWrapper;
use ai::{commit, config};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
  let stdout = io::stdout().into_raw_mode().unwrap();
  let mut stdin = termion::async_stdin().keys();
  let (tx, mut rx) = mpsc::channel(100);

  tokio::spawn(async move {
    loop {
      if let Some(key) = stdin.next() {
        let key = key.unwrap();
        match key {
          // Exit on 'q'
          termion::event::Key::Char('q') => break,
          // Ignore Enter and other keys
          _ => {}
        }
      }
    }
  });

  let logger = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).build();
  let multi = MultiProgress::new();

  LogWrapper::new(multi.clone(), logger).try_init().unwrap();

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

  tokio::spawn(async move {
    for _ in 0..100 {
      if let Some(_) = rx.try_recv().ok() {
        // Handle received input (if any)
      }
      // Simulate work
      pb.inc(1);
      sleep(Duration::from_millis(100)).await;
    }
  });

  let repo = Repository::open_from_env().context("Failed to open repository")?;

  // Get the tree from the commit if the sha1 is provided
  // The sha1 is provided when the user is amending a commit
  let tree = match args.sha1.as_deref() {
    // git commit --amend
    Some("HEAD") => repo.head().ok().and_then(|head| head.peel_to_tree().ok()),
    // git ??
    Some(sha1) => repo.find_commit(sha1.parse()?).ok().and_then(|commit| commit.tree().ok()),
    // git commit
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

  pb.finish_with_message("Done");
  multi.remove(&pb);
  writeln!(stdout, "\n").unwrap();

  Ok(())
}
