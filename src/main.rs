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
  #[clap(long, default_value = "false", help = "git add .")]
  all: bool,

  #[clap(short, long, help = "Enables verbose logging", default_value = "false")]
  verbose: bool
}

lazy_static! {
  static ref MAX_CHARS: usize = dotenv!("MAX_CHARS").parse::<usize>().unwrap();
}


#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
  env_logger::init();
  dotenv().ok();

  let cli = Cli::parse();

  if cli.verbose {
    std::env::set_var("RUST_LOG", "debug");
  }

  "this is blue".blue();
  "this is red".red();
  "this is red on blue".red().on_blue();
  "this is also red on blue".on_blue().red();
  "you can use truecolor values too!".truecolor(0, 255, 136);
  "background truecolor also works :)".on_truecolor(135, 28, 167);
  "bright colors are welcome as well".on_bright_blue().bright_red();
  "you can also make bold comments".bold();
  println!("{} {} {}", "or use".cyan(), "any".italic().yellow(), "string type".cyan());
  "or change advice. This is red".yellow().blue().red();
  "or clear things up. This is default color and style".red().bold().clear();
  "purple and magenta are the same".purple().magenta();
  "and so are normal and clear".normal().clear();
  "you can specify color by string".color("blue").on_color("red");
  String::from("this also works!").green().bold();
  format!("{:30}", "format works as expected. This will be padded".blue());
    format!("{:.3}", "and this will be green but truncated to 3 chars".green());
    
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
