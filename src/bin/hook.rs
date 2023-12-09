// Hook: prepare-commit-msg

#![feature(assert_matches)]

#[cfg(not(mock))]
use ai::hook::Args;
use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();
  let args = Args::parse();
  Ok(ai::hook::run(&args).await?)
}
