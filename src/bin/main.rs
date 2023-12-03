#![feature(lazy_cell)]

// pub mod chat;
// pub mod git;

use ai::chat::generate_commit_message;
use log::{debug, LevelFilter};
use lazy_static::lazy_static;
use dotenv_codegen::dotenv;
use anyhow::Result;
use dotenv::dotenv;
use clap::Parser;
use colored::*;
use ai::git::Repo;

extern crate dotenv_codegen;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
  #[clap(long, default_value = "false", help = "git add .")]
  all: bool,

  #[clap(short, long, help = "Enables verbose logging", default_value = "false")]
  verbose: bool
}

lazy_static! {
  static ref MAX_CHARS: usize = dotenv!("MAX_CHARS").parse::<usize>().unwrap();
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  dotenv().ok();

  let cli = Cli::parse();

  if cli.verbose {
    env_logger::builder()
      .filter_level(LevelFilter::Debug)
      .format_target(false)
      .format_timestamp(None)
      .init();
    debug!("Verbose logging enabled");
  }

  let repo = Repo::new()?;

  if cli.all {
    repo.add_all()?;
  }

  let (diff, files) = repo.diff(*MAX_CHARS)?;
  let message = generate_commit_message(diff).await?;
  let oid = repo.commit(&message)?;

  println!("{} [{:.7}] {}: ", "🤖", oid.to_string().yellow(), message.green().italic());
  for file in files {
    println!("   {}", file.white());
  }

  Ok(())
}
