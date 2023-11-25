#![feature(lazy_cell)]

pub mod git;
pub mod chat;

use dotenv::dotenv;
use anyhow::Result;
use colored::*;
use clap::Parser;
use lazy_static::lazy_static;
use dotenv_codegen::dotenv;
use chat::generate_commit_message;
use git::Repo;

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
  env_logger::init();
  dotenv().ok();

  let cli = Cli::parse();

  if cli.verbose {
    std::env::set_var("RUST_LOG", "debug");
  }

  let repo = Repo::new()?;
  let (diff, files) = repo.diff(*MAX_CHARS)?;
  let message = generate_commit_message(diff).await?;
  let oid = repo.commit(&message, cli.all)?;

  println!("{} [{:.7}] {}: ", "🤖", oid.to_string().yellow(), message.green().italic());
  for file in files {
    println!("   {}", file.white());
  }

  Ok(())
}
