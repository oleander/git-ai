mod install;
mod uninstall;
mod config;

use anyhow::{Context, Result};
use dotenv::dotenv;
use clap::{Arg, Command};

fn cli() -> Command {
  Command::new("git-ai")
    .about("A git extension that uses OpenAI to generate commit messages")
    .subcommand_required(true)
    .arg_required_else_help(true)
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
            .subcommand(
              Command::new("timeout")
                .about("Sets the timeout for the OpenAI API")
                .arg(Arg::new("VALUE").required(true).index(1))
            )
            .subcommand(
              Command::new("model")
                .about("Sets the model to use")
                .arg(Arg::new("VALUE").required(true).index(1))
            )
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
      match args.subcommand() {
        Some(("set", args)) => {
          let mut app = config::App::new()?;
          match args.subcommand() {
            Some(("timeout", args)) => {
              app.timeout = args
                .get_one::<String>("VALUE")
                .context("Failed to get timeout value")?
                .parse::<usize>()
                .context("Failed to parse timeout value")?;
            },
            Some(("model", args)) => {
              app.model = args.get_one::<String>("VALUE").context("Failed to get model value")?.to_owned();
            },
            _ => unreachable!()
          }
          app.save()?;
        },
        _ => unreachable!()
      }
    },
    _ => unreachable!()
  }

  Ok(())
}
