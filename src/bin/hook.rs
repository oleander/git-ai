// Hook: prepare-commit-msg

use std::time::Duration;

use ai::commit::Session;
use termion::event::Key;
use git2::{Oid, Repository};
use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use tokio::time::sleep;
use tokio::signal;
use ai::{commit, config};
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
    // git ???
    Some(sha1) => repo.find_object(Oid::from_str(sha1)?, None).ok().and_then(|obj| obj.peel_to_tree().ok())
  };

  let patch = repo.to_patch(tree, max_tokens).context("Failed to get patch")?;

  if patch.is_empty() {
    Err(HookError::EmptyDiffOutput)?;
  }

  let session = Session::load_from_repo(&repo).await.unwrap();
  let respomse = commit::generate(patch.to_string(), session.into(), pb.clone().into()).await?;
  let commit = respomse.response.trim();
  args.commit_msg_file.write(commit.trim().to_string()).unwrap();
  respomse.session.save_to_repo(&repo).await.unwrap();

  let pb1 = pb.clone();
  tokio::select! {
    _ = signal::ctrl_c() => {
      console::Term::stdout().show_cursor().expect("Failed to show cursor");
      std::process::exit(1);
    }

    _ = read_input(pb1.clone()) => {
      pb1.finish_and_clear();
    }
  }

  log::debug!("Commit message generated successfully");
  pb1.finish_and_clear();

  Ok(())
}

async fn read_input(pb: ProgressBar) -> tokio::io::Result<i32> {
  let _stdout = std::io::stdout().into_raw_mode().unwrap();
  let mut stdin = termion::async_stdin().keys();

  loop {
    match stdin.next() {
      // Ctrl+C pressed: exit the program
      Some(Ok(Key::Ctrl('c'))) => {
        return Ok(1);
      },

      // Enter pressed: render empty line before progress bar
      Some(Ok(Key::Char('\n'))) => {
        pb.println("");
      },

      // Any other key pressed
      _ => {
        sleep(Duration::from_millis(50)).await;
      }
    }
  }
}
