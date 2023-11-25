#![feature(lazy_cell)]

pub mod git;
pub mod chat;

use dotenv::dotenv;
use anyhow::Result;
use colored::*;
use clap::Parser;
use lazy_static::lazy_static;
use dotenv_codegen::dotenv;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
  #[clap(long, help = "Enables debug logging", default_value = "false")]
  debug: bool,

  #[clap(long, default_value = "false", help = "git add .")]
  all: bool
}

lazy_static! {
  static ref MAX_CHARS: usize = dotenv!("MAX_CHARS").parse::<usize>().unwrap();
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
  let (diff, files) = repo.diff(*MAX_CHARS)?;
  let message = chat::suggested_commit_message(diff).await?;
  let oid = repo.commit(&message, cli.all)?;

  println!("{} [{:.7}] {}: ", "🤖", oid.to_string().yellow(), message.green());
  for file in files {
    println!("   {}", file.white());
  }

  Ok(())
}
