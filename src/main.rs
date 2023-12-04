mod install;
mod uninstall;

use anyhow::Result;
use dotenv::dotenv;
use clap::Command;

fn cli() -> Command {
  Command::new("git-ai")
    .about("A git extension that uses OpenAI to generate commit messages")
    .subcommand_required(true)
    .arg_required_else_help(true)
    .allow_external_subcommands(true)
    .subcommand(Command::new("install").about("Installs the git-ai hook"))
    .subcommand(Command::new("uninstall").about("Uninstalls the git-ai hook").arg_required_else_help(true))
}

#[tokio::main]
async fn main() -> Result<()> {
  dotenv().ok();

  let args = cli().get_matches();

  match args.subcommand() {
    Some(("install", _)) => {
      install::run()?;
    },
    Some(("uninstall", _)) => {
      uninstall::run()?;
    },
    _ => {
      log::info!("Running git-ai...");
    }
  }

  Ok(())
}
