mod config;
mod filesystem;

use clap::{Parser, Subcommand};
use anyhow::Result;
use dotenv::dotenv;

use crate::config::App;
use crate::filesystem::Filesystem;

#[derive(Parser)]
#[command(name = "git-ai", about = "A git extension that uses OpenAI to generate commit messages")]
enum Cli {
  #[command(subcommand)]
  Hook(HookSubcommand),
  #[command(subcommand)]
  Config(ConfigSubcommand)
}

#[derive(Subcommand)]
enum HookSubcommand {
  #[command(about = "Installs the git-ai hook")]
  Install,
  #[command(about = "Uninstalls the git-ai hook")]
  Uninstall,
  #[command(about = "Reinstalls the git-ai hook")]
  Reinstall
}

#[derive(Subcommand)]
enum ConfigSubcommand {
  #[command(subcommand)]
  Set(SetSubcommand),

  #[command(about = "Resets the internal configuration to the default values")]
  Reset
}

#[derive(Subcommand)]
enum SetSubcommand {
  #[command(about = "Sets the model to use")]
  Model(Model),

  #[command(about = "Sets the maximum number of tokens to use for the diff")]
  MaxTokens {
    #[arg(help = "The maximum number of tokens", value_name = "max-tokens")]
    max_tokens: usize
  },

  #[command(about = "Sets the maximum length of the commit message")]
  MaxCommitLength {
    #[arg(help = "The maximum length of the commit message", value_name = "max-commit-length")]
    max_commit_length: usize
  },

  #[command(about = "Sets the OpenAI API key")]
  OpenaiApiKey {
    #[arg(help = "The OpenAI API key", value_name = "VALUE")]
    value: String
  }
}

#[derive(Parser)]
struct Model {
  #[arg(help = "The value to set", value_name = "VALUE")]
  value: String
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
  println!("🔗 Hook symlinked successfully to \x1B[3m{hook_file}\x1B[0m");

  Ok(())
}

fn run_uninstall() -> Result<()> {
  let fs = Filesystem::new()?;
  let hook_file = fs.prepare_commit_msg_path()?;

  if hook_file.exists() {
    hook_file.delete()?;
    println!("🗑️  Hook uninstalled successfully from \x1B[3m{hook_file}\x1B[0m");
  } else {
    println!("⚠️  No hook found at \x1B[3m{hook_file}\x1B[0m");
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
  println!("✅ Model set to: {value}");
  Ok(())
}

fn run_config_max_tokens(max_tokens: usize) -> Result<()> {
  let mut app = App::new()?;
  app.update_max_tokens(max_tokens)?;
  println!("✅ Max tokens set to: {max_tokens}");
  Ok(())
}

fn run_config_max_commit_length(max_commit_length: usize) -> Result<()> {
  let mut app = App::new()?;
  app.update_max_commit_length(max_commit_length)?;
  println!("✅ Max commit length set to: {max_commit_length}");
  Ok(())
}

fn run_config_openai_api_key(value: String) -> Result<()> {
  let mut app = App::new()?;
  app.update_openai_api_key(value)?;
  println!("✅ OpenAI API key updated");
  Ok(())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
  // Load environment variables from .env file if present
  dotenv().ok();

  // Initialize logging with debug level in debug builds
  #[cfg(debug_assertions)]
  {
    if std::env::var("RUST_LOG").is_err() {
      std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();
    println!("Debug build: Performance profiling enabled");
  }

  let args = Cli::parse();

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
          },
      },
  }

  Ok(())
}
