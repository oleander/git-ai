use std::io::{self, Write};
use std::time::Duration;

use git2::Repository;
use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio::{select, time};
use ai::hook::*;
use ai::{commit, config};
use env_logger;
use indicatif_log_bridge::LogWrapper;

#[tokio::main]
async fn main() -> Result<()> {
  // env_logger::init();
  let logger = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).build();
  let multi = MultiProgress::new();

  LogWrapper::new(multi.clone(), logger).try_init().unwrap();

  let mut stdout = io::stdout().into_raw_mode()?;
  let mut stdin = termion::async_stdin().keys();
  let (tx, mut rx) = mpsc::channel(32);

  tokio::spawn(async move {
    loop {
      if let Some(key) = stdin.next() {
        match key {
          Ok(termion::event::Key::Ctrl('c')) => {
            let _ = tx.send(()).await;
            break;
          },
          _ => {}
        }
      }
      sleep(Duration::from_millis(50)).await;
    }
  });

  let args = Args::parse();

  if args.commit_type.is_some() {
    return Ok(());
  }

  let pb = ProgressBar::new_spinner();
  pb.enable_steady_tick(Duration::from_millis(150));
  pb.set_message("Generating commit message...");

  let repo = Repository::open_from_env().context("Failed to open repository")?;
  let tree = match args.sha1.as_deref() {
    Some("HEAD") => repo.head().ok().and_then(|head| head.peel_to_tree().ok()),
    Some(sha1) => panic!("sha1: {}", sha1),
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

  // Separate blocking task for progress bar updates
  let progress_task = tokio::task::spawn_blocking(move || {
    pb.set_style(
      ProgressStyle::default_spinner()
        .tick_strings(&["-", "\\", "|", "/"])
        .template("{spinner:.blue} {msg}")
        .expect("Failed to set progress bar style")
    );
    for i in 0..100 {
      pb.set_position(i);
      std::thread::sleep(Duration::from_millis(100));
    }
    pb.finish_with_message("Done");
    multi.remove(&pb);
  });

  // Wait for either the progress task to complete or an exit signal from the input handler
  select! {
      _ = progress_task => {},
      _ = rx.recv() => {
          log::info!("Received exit signal");
          // pb.finish_with_message("Done");
          // multi.remove(&pb);
          writeln!(stdout, "").unwrap(); // Ensure to leave the terminal in a clean state
          std::process::exit(0);
      },
  }

  writeln!(stdout, "").unwrap(); // Ensure to leave the terminal in a clean state

  Ok(())
}
