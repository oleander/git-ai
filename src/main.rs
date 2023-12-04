mod install;
mod uninstall;

use anyhow::Result;
use clap::Arg;
use dotenv::dotenv;
use clap::{Command, arg};

fn cli() -> Command {
  Command::new("git-ai")
    .about("A git extension that uses OpenAI to generate commit messages")
    .subcommand_required(true)
    .arg_required_else_help(true)
    .allow_external_subcommands(true)
    .subcommand(Command::new("install").about("Installs the git-ai hook"))
    .subcommand(Command::new("uninstall").about("Uninstalls the git-ai hook"))
    .subcommand(
      Command::new("config")
        .about("Sets or gets configuration values")
        .subcommand(
          Command::new("set")
            .about("Sets a configuration value")
            .arg(arg!(<KEY> "The configuration key"))
            .arg(arg!(<VALUE> "The configuration value"))
        )
        .subcommand(
          Command::new("get")
            .about("Gets a configuration value")
            .arg(Arg::new("key").help("The configuration key").required(true).index(1))
        )
    )
}
#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();
  dotenv().ok();

  let args = cli().get_matches();

  match args.subcommand() {
    Some(("install", _)) => {
      install::run()?;
    },
    Some(("uninstall", _)) => {
      uninstall::run()?;
    },
    Some(("config", args)) => {
      if let Some(matches) = args.subcommand_matches("set") {
        let key = matches.get_one::<String>("KEY").expect("required");
        let value = matches.get_one::<String>("VALUE").expect("required");
        log::info!("Setting config key {} to {}", key, value);
        // config::set(key, value)?;
      }
    },
    _ => {
      println!("No subcommand was used");
    }
  }

  Ok(())
}
