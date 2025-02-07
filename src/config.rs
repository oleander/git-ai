use std::io::Write;
use std::path::PathBuf;
use std::fs::File;

use serde::{Deserialize, Serialize};
use config::{Config, FileFormat};
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use console::Emoji;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
  pub provider:          Option<String>,
  pub openai:            OpenAIConfig,
  pub ollama:            OllamaConfig,
  pub max_tokens:        Option<usize>,
  pub max_commit_length: Option<usize>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenAIConfig {
  pub api_key: Option<String>,
  pub model:   Option<String>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaConfig {
  pub model: Option<String>,
  pub host:  Option<String>,
  pub port:  Option<u16>
}

impl Default for AppConfig {
  fn default() -> Self {
    Self {
      provider:          Some("openai".to_string()),
      openai:            OpenAIConfig { api_key: None, model: Some("gpt-4".to_string()) },
      ollama:            OllamaConfig {
        model: Some("llama2".to_string()),
        host:  Some("localhost".to_string()),
        port:  Some(11434)
      },
      max_tokens:        Some(4000),
      max_commit_length: Some(72)
    }
  }
}

lazy_static! {
  pub static ref CONFIG_DIR: PathBuf = home::home_dir().unwrap().join(".config/git-ai");
  pub static ref APP: AppConfig = load_config().unwrap_or_default();
  pub static ref CONFIG_PATH: PathBuf = CONFIG_DIR.join("config.ini");
}

impl AppConfig {
  pub fn new() -> Result<Self> {
    dotenv::dotenv().ok();

    if !CONFIG_DIR.exists() {
      std::fs::create_dir_all(CONFIG_DIR.to_str().unwrap()).context("Failed to create config directory")?;
      File::create(CONFIG_PATH.to_str().unwrap()).context("Failed to create config file")?;
    } else if !CONFIG_PATH.exists() {
      File::create(CONFIG_PATH.to_str().unwrap()).context("Failed to create config file")?;
    }

    let config = Config::builder()
      .add_source(config::Environment::with_prefix("APP").try_parsing(true))
      .add_source(config::File::new(CONFIG_PATH.to_str().unwrap(), FileFormat::Ini))
      .set_default("language", "en")?
      .set_default("timeout", 30)?
      .set_default("max_commit_length", 72)?
      .set_default("max_tokens", 2024)?
      .set_default("model", "gpt-4o")?
      .set_default("openai_api_key", "<PLACE HOLDER FOR YOUR API KEY>")?
      .build()?;

    config
      .try_deserialize()
      .context("Failed to deserialize existing config. Please run `git ai config reset` and try again")
  }

  pub fn save(&self) -> Result<()> {
    let contents = serde_ini::to_string(&self).context(format!("Failed to serialize config: {:?}", self))?;
    let mut file = File::create(CONFIG_PATH.to_str().unwrap()).context("Failed to create config file")?;
    file
      .write_all(contents.as_bytes())
      .context("Failed to write config file")
  }
}

pub fn run_reset() -> Result<()> {
  let config = AppConfig::default();
  save_config(&config)
}

pub fn run_provider(provider: String) -> Result<()> {
  let mut config = load_config()?;
  match provider.as_str() {
    "openai" | "ollama" => {
      config.provider = Some(provider);
      save_config(&config)
    }
    _ => Err(anyhow::anyhow!("Invalid provider: {}", provider))
  }
}

pub fn run_openai_config(api_key: Option<String>, model: Option<String>) -> Result<()> {
  let mut config = load_config()?;
  if let Some(key) = api_key {
    config.openai.api_key = Some(key);
  }
  if let Some(model) = model {
    config.openai.model = Some(model);
  }
  save_config(&config)
}

pub fn run_ollama_config(model: Option<String>, host: Option<String>, port: Option<u16>) -> Result<()> {
  let mut config = load_config()?;
  if let Some(model) = model {
    config.ollama.model = Some(model);
  }
  if let Some(host) = host {
    config.ollama.host = Some(host);
  }
  if let Some(port) = port {
    config.ollama.port = Some(port);
  }
  save_config(&config)
}

fn load_config() -> Result<AppConfig> {
  if !CONFIG_PATH.exists() {
    return Ok(AppConfig::default());
  }

  let contents = std::fs::read_to_string(&*CONFIG_PATH).context("Failed to read config file")?;

  serde_ini::from_str(&contents).context("Failed to parse config file")
}

fn save_config(config: &AppConfig) -> Result<()> {
  if let Some(parent) = CONFIG_PATH.parent() {
    std::fs::create_dir_all(parent).context("Failed to create config directory")?;
  }

  let contents = serde_ini::to_string(config).context("Failed to serialize config")?;

  std::fs::write(&*CONFIG_PATH, contents).context("Failed to write config file")?;

  println!("{} Configuration updated!", Emoji("✨", ":-)"));
  Ok(())
}
