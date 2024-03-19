mod uninstall;
mod install;
mod config;
mod examples;

use clap::{Arg, Command};
use anyhow::Result;
use dotenv::dotenv;

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
              Command::new("model").about("Sets the model to use").arg(
                Arg::new("<VALUE>")
                  .required(true)
                  .index(1)
                  .value_parser(clap::builder::NonEmptyStringValueParser::new())
              )
            )
            .subcommand(
              Command::new("language").about("Sets the language to use").arg(
                Arg::new("<VALUE>")
                  .required(true)
                  .index(1)
                  .value_parser(clap::builder::NonEmptyStringValueParser::new())
              )
            )
            .subcommand(
              Command::new("max-diff-tokens")
                .about("Sets the maximum number of tokens to use for the diff")
                .arg(
                  Arg::new("max-diff-tokens")
                    .required(true)
                    .index(1)
                    .value_parser(clap::value_parser!(usize))
                )
            )
            .subcommand(
              Command::new("max-length")
                .about("Sets the maximum length of the commit message")
                .arg(
                  Arg::new("max-length")
                    .required(true)
                    .index(1)
                    .value_parser(clap::value_parser!(usize))
                )
            )
            .subcommand(
              Command::new("openai-api-key").about("Sets the OpenAI API key").arg(
                Arg::new("<VALUE>")
                  .required(true)
                  .index(1)
                  .value_parser(clap::builder::NonEmptyStringValueParser::new())
              )
            )
        )
    )
    .subcommand(Command::new("examples").about("Runs examples of generated commit messages"))
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
          config::run(args)?;
        },
        _ => unreachable!()
      }
    },
    Some(("examples", args)) => {
      examples::run(args).await?;
    },
    _ => unreachable!()
  }

  Ok(())
}
