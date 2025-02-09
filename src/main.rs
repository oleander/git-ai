mod config;
mod filesystem;

use structopt::StructOpt;
use anyhow::Result;
use dotenv::dotenv;

use crate::config::App;
use crate::filesystem::Filesystem;

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
  Reset
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
  },

  #[structopt(about = "OpenAI configuration")]
  OpenAI(OpenAISubcommand)
}

#[derive(StructOpt)]
enum OpenAISubcommand {
  #[structopt(about = "Sets the OpenAI API URL")]
  Url {
    #[structopt(env = "OPENAI_URL", default_value = "https://api.openai.com/v1")]
    url: String
  }
}

#[derive(StructOpt)]
struct Model {
  #[structopt(help = "The value to set", name = "VALUE")]
  value: String
}

#[derive(Debug, StructOpt)]
#[structopt(name = "git-ai")]
pub struct Args {
  #[structopt(subcommand)]
  #[allow(dead_code)]
  cmd: Command
}

#[derive(Debug, StructOpt)]
pub enum Command {
  #[structopt(name = "optimize")]
  Optimize {
    #[structopt(long, default_value = "resources/prompt.md")]
    prompt_file: String,

    #[structopt(long, default_value = "stats.json")]
    stats_file: String,

    #[structopt(long, default_value = "tmp")]
    temp_dir: String,

    #[structopt(long, default_value = "100")]
    iterations: u32,

    #[structopt(long, default_value = "0.8")]
    threshold: f32,

    #[structopt(long, default_value = "ai")]
    scoring_mode: String,

    #[structopt(long)]
    verbose: bool
  }
}

// Hook installation functions
fn run_install() -> Result<()> {
  let fs = Filesystem::new()?;
  let hook_bin = fs.git_ai_hook_bin_path()?;
  let hook_file = fs.prepare_commit_msg_path()?;

  if hook_file.exists() {
    hook_file.delete()?;
  }

  hook_file.symlink(&hook_bin)?;
  println!("🔗 Hook symlinked successfully to \x1B[3m{}\x1B[0m", hook_file);

  Ok(())
}

fn run_uninstall() -> Result<()> {
  let fs = Filesystem::new()?;
  let hook_file = fs.prepare_commit_msg_path()?;

  if hook_file.exists() {
    hook_file.delete()?;
    println!("🗑️  Hook uninstalled successfully from \x1B[3m{}\x1B[0m", hook_file);
  } else {
    println!("⚠️  No hook found at \x1B[3m{}\x1B[0m", hook_file);
  }

  Ok(())
}

fn run_reinstall() -> Result<()> {
  run_uninstall()?;
  run_install()?;
  Ok(())
}

// Config management functions
fn run_config_reset() -> Result<()> {
  let config_dir = dirs::config_dir()
    .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
    .join("git-ai");

  if config_dir.exists() {
    std::fs::remove_dir_all(&config_dir)?;
    println!("🗑️  Configuration reset successfully");
  } else {
    println!("⚠️  No configuration found to reset");
  }

  Ok(())
}

fn run_config_model(value: String) -> Result<()> {
  let mut app = App::new()?;
  app.update_model(value.clone())?;
  println!("✅ Model set to: {}", value);
  Ok(())
}

fn run_config_max_tokens(max_tokens: usize) -> Result<()> {
  let mut app = App::new()?;
  app.update_max_tokens(max_tokens)?;
  println!("✅ Max tokens set to: {}", max_tokens);
  Ok(())
}

fn run_config_max_commit_length(max_commit_length: usize) -> Result<()> {
  let mut app = App::new()?;
  app.update_max_commit_length(max_commit_length)?;
  println!("✅ Max commit length set to: {}", max_commit_length);
  Ok(())
}

fn run_config_openai_api_key(value: String) -> Result<()> {
  let mut app = App::new()?;
  app.update_openai_api_key(value)?;
  println!("✅ OpenAI API key updated");
  Ok(())
}

fn run_config_openai_host(value: String) -> Result<()> {
  let mut app = App::new()?;
  app.update_openai_host(value)?;
  println!("✅ OpenAI host URL updated");
  Ok(())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
  dotenv().ok();

  let args = Cli::from_args();

  match args {
    Cli::Hook(sub) =>
      match sub {
        HookSubcommand::Install => {
          run_install()?;
        }
        HookSubcommand::Uninstall => {
          run_uninstall()?;
        }
        HookSubcommand::Reinstall => {
          run_reinstall()?;
        }
      },
    Cli::Config(config) =>
      match config {
        ConfigSubcommand::Reset => {
          run_config_reset()?;
        }

        ConfigSubcommand::Set(set) =>
          match set {
            SetSubcommand::Model(model) => {
              run_config_model(model.value)?;
            }
            SetSubcommand::MaxTokens { max_tokens } => {
              run_config_max_tokens(max_tokens)?;
            }
            SetSubcommand::MaxCommitLength { max_commit_length } => {
              run_config_max_commit_length(max_commit_length)?;
            }
            SetSubcommand::OpenaiApiKey { value } => {
              run_config_openai_api_key(value)?;
            }
            SetSubcommand::OpenAI(openai) =>
              match openai {
                OpenAISubcommand::Url { url } => {
                  run_config_openai_host(url)?;
                }
              },
          },
      },
  }

  Ok(())
}
