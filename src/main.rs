mod install;
mod uninstall;
mod config;

use anyhow::Result;
use dotenv::dotenv;
use clap::{Arg, Command};

fn cli() -> Command {
  Command::new("git-ai")
    .about("A git extension that uses OpenAI to generate commit messages")
    .subcommand_required(true)
    .arg_required_else_help(true)
    // .allow_external_subcommands(true)
    .subcommand(
      Command::new("hook")
        .about("Installs the git-ai hook")
        .subcommand(Command::new("install").about("Installs the git-ai hook"))
        .subcommand(Command::new("uninstall").about("Uninstalls the git-ai hook"))
    )
    .subcommand(
      Command::new("config")
        .about("Sets or gets configuration values")
        .subcommand(
          Command::new("set")
            .about("Sets a configuration value")
            .arg(Arg::new("KEY").required(true).index(1))
            .arg(Arg::new("VALUE").required(true).index(2))
        )
        .subcommand(
          Command::new("get")
            .about("Gets a configuration value")
            .arg(Arg::new("KEY").required(true).index(1))
        )
    )
}
#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();
  dotenv().ok();

  let args = cli().get_matches();

  match args.subcommand() {
    Some(("hook", sub)) => {
      match sub.subcommand() {
        Some(("install", _)) => {
          install::run()?;
        },
        Some(("uninstall", _)) => {
          uninstall::run()?;
        },
        _ => unreachable!()
      }
    },
    Some(("config", args)) => {
      if let Some(matches) = args.subcommand_matches("set") {
        let key = matches.get_one::<String>("KEY").expect("required");
        let value = matches.get_one::<String>("VALUE").expect("required");
        log::info!("Setting config key {} to {}", key, value);
        config::set(key, value.as_str())?;
      } else if let Some(matches) = args.subcommand_matches("get") {
        let key = matches.get_one::<String>("KEY").expect("required");
        let value: String = config::get(key)?;
        log::info!("Config key {} is set to {}", key, value);
      }
    },
    _ => unreachable!()
  }

  Ok(())
}
