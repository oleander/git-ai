use std::io::{self, Write};
use std::time::Duration;

use git2::Repository;
use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use tokio::sync::mpsc;
use std::io::BufReader;
use tokio::time::sleep;
use tokio::{select, time};
use ai::hook::*;
use ai::{commit, config};
use env_logger;
use indicatif_log_bridge::LogWrapper;
use crossterm::terminal;

#[tokio::main]
async fn main() -> Result<()> {
  let mut stdin = termion::async_stdin().keys();



  tokio::spawn(async move {
    match stdin.next() {
      Some(Ok(key)) => {
        if termion::event::Key::Ctrl('c') == key {
          std::process::exit(1);
        }
      }
      _ => {}
    }
  });

  let args = Args::parse();

  if args.commit_type.is_some() {
    return Ok(());
  }

  let pb = ProgressBar::new_spinner();
  pb.enable_steady_tick(Duration::from_millis(150));
  pb.set_message("Generating commit message...");
  pb.set_style(
    ProgressStyle::default_spinner()
      .tick_strings(&["-", "\\", "|", "/"])
      .template("{spinner:.blue} {msg}")
      .expect("Failed to set progress bar style")
  );

  let repo = Repository::open_from_env().context("Failed to open repository")?;
  let tree = match args.sha1.as_deref() {
    Some("HEAD") => repo.head().ok().and_then(|head| head.peel_to_tree().ok()),
    Some(sha1) => repo.find_object(git2::Oid::from_str(sha1)?, None).ok().and_then(|obj| obj.peel_to_tree().ok()),
    None => repo.head().ok().and_then(|head| head.peel_to_tree().ok())
  };

  let max_tokens = config::APP.max_diff_tokens;
  let patch = repo.to_patch(tree, max_tokens).context("Failed to get patch")?;

  if patch.is_empty() {
    return Err(anyhow::Error::new(HookError::EmptyDiffOutput));
  }

  let commit_message = commit::generate(patch.to_string()).await?;
  args
    .commit_msg_file
    .write(commit_message.trim().to_string())
    .context("Failed to write commit message")?;

  Ok(())
}
