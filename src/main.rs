mod uninstall;
mod install;
mod reinstall;
mod config;

use structopt::StructOpt;
use anyhow::Result;
use dotenv::dotenv;

#[derive(StructOpt)]
#[structopt(name = "git-ai", about = "A git extension that uses AI to generate commit messages")]
enum Cli {
  #[structopt(about = "Installs the git-ai hook")]
  Hook(HookSubcommand),
  #[structopt(about = "Configure git-ai settings")]
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
  #[structopt(name = "provider", about = "Configure provider settings")]
  Provider(ProviderCommand),
  #[structopt(name = "openai", about = "Configure OpenAI settings")]
  OpenAI(OpenAICommand),
  #[structopt(name = "ollama", about = "Configure Ollama settings")]
  Ollama(OllamaCommand),
  #[structopt(name = "reset", about = "Reset configuration to defaults")]
  Reset
}

#[derive(StructOpt)]
#[structopt(about = "Configure AI provider")]
struct ProviderCommand {
  #[structopt(name = "name", help = "Provider to use (openai or ollama)", possible_values = &["openai", "ollama"])]
  provider: String
}

#[derive(StructOpt)]
#[structopt(about = "Configure OpenAI settings")]
struct OpenAICommand {
  #[structopt(long, help = "OpenAI API key")]
  api_key: Option<String>,
  #[structopt(long, help = "Model to use (e.g. gpt-4, gpt-4-turbo-preview)")]
  model:   Option<String>
}

#[derive(StructOpt)]
#[structopt(about = "Configure Ollama settings")]
struct OllamaCommand {
  #[structopt(long, help = "Model to use (e.g. llama2, codellama)")]
  model: Option<String>,
  #[structopt(long, help = "Host address (default: localhost)")]
  host:  Option<String>,
  #[structopt(long, help = "Port number (default: 11434)")]
  port:  Option<u16>
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
  dotenv().ok();

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

        ConfigSubcommand::Provider(provider) => {
          config::run_provider(provider.provider)?;
        }

        ConfigSubcommand::OpenAI(openai) => {
          config::run_openai_config(openai.api_key, openai.model)?;
        }

        ConfigSubcommand::Ollama(ollama) => {
          config::run_ollama_config(ollama.model, ollama.host, ollama.port)?;
        }
      },
  }

  Ok(())
}
