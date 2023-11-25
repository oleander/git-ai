#![feature(lazy_cell)]

pub mod git;
pub mod chat;

use std::os::unix::process::CommandExt;
use std::process::{exit, Command};
use dotenv::dotenv;
use anyhow::Result;
use clap::Parser;
use log::error;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
  #[clap(long, help = "Enables debug logging", default_value = "false")]
  debug: bool,

  #[clap(long, default_value = "false", help = "git add .")]
  all: bool
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
  env_logger::init();
  dotenv().ok();

  let cli = Cli::parse();

  if cli.debug {
    std::env::set_var("RUST_LOG", "info");
  }

  if let Err(e) = git::repo().commit(cli.all).await {
    error!("Failed to commit: {}", e);
    exit(1);
  }

  Command::new("git").args(&["--no-pager", "log", "-1", "--name-only"]).exec();

  Ok(())
}
