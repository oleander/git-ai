mod uninstall;
mod install;
mod reinstall;
mod config;
mod wizard;
pub mod model;

use structopt::StructOpt;
use anyhow::Result;
use dotenv::dotenv;

#[derive(StructOpt)]
#[structopt(name = "git-ai", about = "A git extension that uses OpenAI to generate commit messages")]
enum Cli {
  #[structopt(about = "Installs the git-ai hook")]
  Hook(HookSubcommand),
  #[structopt(about = "Sets or gets configuration values")]
  Config(ConfigSubcommand)
}

#[derive(StructOpt)]
enum HookSubcommand {
  #[structopt(about = "Installs the git-ai hook")]
  Install,
  #[structopt(about = "Uninstalls the git-ai hook")]
  Uninstall,
  #[structopt(about = "Reinstalls the git-ai hook")]
  Reinstall
}

#[derive(StructOpt)]
enum ConfigSubcommand {
  #[structopt(about = "Sets a configuration value")]
  Set(SetSubcommand),

  #[structopt(about = "Resets the internal configuration to the default values")]
  Reset,

  #[structopt(about = "Run the interactive configuration wizard")]
  Wizard
}

#[derive(StructOpt)]
enum SetSubcommand {
  #[structopt(about = "Sets the model to use")]
  Model(Model),

  #[structopt(about = "Sets the maximum number of tokens to use for the diff")]
  MaxTokens {
    #[structopt(help = "The maximum number of tokens", name = "max-tokens")]
    max_tokens: usize
  },

  #[structopt(about = "Sets the maximum length of the commit message")]
  MaxCommitLength {
    #[structopt(help = "The maximum length of the commit message", name = "max-commit-length")]
    max_commit_length: usize
  },

  #[structopt(about = "Sets the OpenAI API key")]
  OpenaiApiKey {
    #[structopt(help = "The OpenAI API key", name = "VALUE")]
    value: String
  }
}

#[derive(StructOpt)]
struct Model {
  #[structopt(help = "The value to set", name = "VALUE")]
  value: String
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
  dotenv().ok();

  // Check if setup is needed and run wizard if necessary
  if wizard::needs_setup() {
    wizard::run()?;
  }

  let args = Cli::from_args();

  match args {
    Cli::Hook(sub) =>
      match sub {
        HookSubcommand::Install => {
          install::run()?;
        }
        HookSubcommand::Uninstall => {
          uninstall::run()?;
        }
        HookSubcommand::Reinstall => {
          reinstall::run()?;
        }
      },
    Cli::Config(config) =>
      match config {
        ConfigSubcommand::Reset => {
          config::run_reset()?;
        }
        ConfigSubcommand::Wizard => {
          wizard::run()?;
        }
        ConfigSubcommand::Set(set) =>
          match set {
            SetSubcommand::Model(model) => {
              config::run_model(model.value)?;
            }
            SetSubcommand::MaxTokens { max_tokens } => {
              config::run_max_tokens(max_tokens)?;
            }
            SetSubcommand::MaxCommitLength { max_commit_length } => {
              config::run_max_commit_length(max_commit_length)?;
            }
            SetSubcommand::OpenaiApiKey { value } => {
              config::run_openai_api_key(value)?;
            }
          },
      },
  }

  Ok(())
}
