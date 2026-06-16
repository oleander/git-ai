use structopt::StructOpt;
use anyhow::Result;
use dotenv::dotenv;
use ai::config::AppConfig;
use ai::filesystem::Filesystem;
use ai::{model, openai};

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

  #[structopt(about = "Sets a custom OpenAI-compatible base URL (e.g. a local ollama endpoint)")]
  OpenaiBaseUrl {
    #[structopt(help = "The base URL, e.g. http://localhost:11434/v1", name = "VALUE")]
    value: String
  }
}

#[derive(StructOpt)]
struct Model {
  #[structopt(help = "The value to set", name = "VALUE")]
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

async fn run_config_model(value: String) -> Result<()> {
  let mut app = AppConfig::new()?;

  // Verify the model exists at the configured endpoint before saving. Known and
  // deprecated aliases skip the round-trip; unreachable/unauthorized endpoints
  // warn-and-allow so offline users are not blocked. A definitively-absent model
  // returns an error here and is NOT persisted.
  let known_or_deprecated = model::is_known_or_deprecated(&value);
  openai::verify_model_exists(&app, &value, known_or_deprecated).await?;

  app.update_model(value.clone())?;
  println!("✅ Model set to: {value}");
  Ok(())
}

fn run_config_max_tokens(max_tokens: usize) -> Result<()> {
  let mut app = AppConfig::new()?;
  app.update_max_tokens(max_tokens)?;
  println!("✅ Max tokens set to: {max_tokens}");
  Ok(())
}

fn run_config_max_commit_length(max_commit_length: usize) -> Result<()> {
  let mut app = AppConfig::new()?;
  app.update_max_commit_length(max_commit_length)?;
  println!("✅ Max commit length set to: {max_commit_length}");
  Ok(())
}

fn run_config_openai_api_key(value: String) -> Result<()> {
  let mut app = AppConfig::new()?;
  app.update_openai_api_key(value)?;
  println!("✅ OpenAI API key updated");
  Ok(())
}

fn run_config_openai_base_url(value: String) -> Result<()> {
  let mut app = AppConfig::new()?;
  app.update_openai_base_url(value.clone())?;
  println!("✅ OpenAI base URL set to: {value}");
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
              run_config_model(model.value).await?;
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
            SetSubcommand::OpenaiBaseUrl { value } => {
              run_config_openai_base_url(value)?;
            }
          },
      },
  }

  Ok(())
}
