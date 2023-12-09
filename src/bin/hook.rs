// Hook: prepare-commit-msg

#![feature(assert_matches)]

use indicatif::{ProgressBar, ProgressStyle};
use tokio::time::Duration;
use anyhow::{Context, Result};
use ai::hook::Args;
use clap::Parser;

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

  Ok(ai::hook::run(&args).await?)
}
