#![feature(lazy_cell)]

pub mod git;
pub mod chat;

use dotenv::dotenv;
use anyhow::Result;
use colored::*;
use clap::Parser;

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

  let repo = git::Repo::new()?;
  let (diff, files) = repo.diff(1000)?;
  let message = chat::suggested_commit_message(diff).await?;
  let oid = repo.commit(&message, cli.all)?;

  println!("{} [{:.7}] {}: ", "🤖".bright_blue(), oid.to_string().yellow(), message.bright_green());
  for file in files {
    println!("   {}", file.white());
  }

  Ok(())
}
