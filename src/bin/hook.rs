use std::io::{self, BufReader, Write};
use std::time::Duration;

use termion::event::Key;
use tokio::io::AsyncReadExt;
use git2::Repository;
use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio::{select, signal, time};
use ai::hook::*;
use ai::{commit, config};
use env_logger;
use indicatif_log_bridge::LogWrapper;
use crossterm::terminal;

async fn read_input(pb: ProgressBar) -> tokio::io::Result<i32> {
  let mut stdin = termion::async_stdin().keys();

  loop {
    match stdin.next() {
      Some(Ok(Key::Ctrl('c'))) => {
        return Ok(1);
      }

      Some(Ok(_)) => {
        pb.abandon();
      }


      Some(Err(e)) => {
        return Ok(1);
      }

      None => {
        sleep(Duration::from_millis(50)).await;
      }
    }
  }
}

#[tokio::main]
async fn main() -> Result<()> {
  let args = Args::parse();

  if args.commit_type.is_some() {
    return Ok(());
  }

  let pb = ProgressBar::new_spinner();
  // pb.set_draw_target(stdin);
  pb.enable_steady_tick(Duration::from_millis(150));
  pb.set_message("Generating commit message...");
  pb.set_style(
    ProgressStyle::default_spinner()
      .tick_strings(&["-", "\\", "|", "/"])
      .template("{spinner:.blue} {msg}")
      .expect("Failed to set progress bar style")
  );

  let process = tokio::spawn(async move {
    let repo = Repository::open_from_env().context("Failed to open repository")?;
    let tree = match args.sha1.as_deref() {
      Some("HEAD") => repo.head().ok().and_then(|head| head.peel_to_tree().ok()),
      Some(sha1) => {
        repo
          .find_object(git2::Oid::from_str(sha1)?, None)
          .ok()
          .and_then(|obj| obj.peel_to_tree().ok())
      },
      None => repo.head().ok().and_then(|head| head.peel_to_tree().ok())
    };

    let max_tokens = config::APP.max_diff_tokens;
    let patch = repo.to_patch(tree, max_tokens).context("Failed to get patch")?;

    if patch.is_empty() {
      return Err(anyhow::Error::new(HookError::EmptyDiffOutput));
    }

    args
      .commit_msg_file
      .write(commit::generate(patch.to_string()).await?.trim().to_string())
      .context("Failed to write commit message")?;


    Ok(())
  });

  tokio::select! {
    _ = signal::ctrl_c() => {
        std::process::exit(1);
    }

    _ = process => {
      pb.finish_and_clear();
    }

    _ = read_input(pb.clone()) => {
      // pb.finish_and_clear();
    }
  }

  Ok(())
}
