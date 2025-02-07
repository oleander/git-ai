use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use console::Emoji;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppConfig {
  #[serde(rename = "provider")]
  pub provider:          Option<String>,
  #[serde(flatten)]
  pub openai:            OpenAIConfig,
  #[serde(flatten)]
  pub ollama:            OllamaConfig,
  #[serde(rename = "max_tokens")]
  pub max_tokens:        Option<usize>,
  #[serde(rename = "max_commit_length")]
  pub max_commit_length: Option<usize>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenAIConfig {
  #[serde(rename = "openai_api_key")]
  pub api_key: Option<String>,
  #[serde(rename = "openai_model")]
  pub model:   Option<String>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaConfig {
  #[serde(rename = "ollama_model")]
  pub model: Option<String>,
  #[serde(rename = "ollama_host")]
  pub host:  Option<String>,
  #[serde(rename = "ollama_port")]
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

impl Default for OpenAIConfig {
  fn default() -> Self {
    Self { api_key: None, model: Some("gpt-4".to_string()) }
  }
}

impl Default for OllamaConfig {
  fn default() -> Self {
    Self {
      model: Some("llama2".to_string()),
      host:  Some("localhost".to_string()),
      port:  Some(11434)
    }
  }
}

lazy_static! {
  pub static ref CONFIG_DIR: PathBuf = home::home_dir().unwrap().join(".config/git-ai");
  pub static ref APP: AppConfig = load_config().unwrap_or_default();
  pub static ref CONFIG_PATH: PathBuf = CONFIG_DIR.join("config.ini");
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

  let mut config = AppConfig::default();

  for line in contents.lines() {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
      continue;
    }

    if let Some((key, value)) = line.split_once('=') {
      let key = key.trim();
      let value = value.trim();

      match key {
        // General settings
        "provider" => config.provider = Some(value.to_string()),
        "max_tokens" => config.max_tokens = value.parse().ok(),
        "max_commit_length" => config.max_commit_length = value.parse().ok(),

        // OpenAI settings
        "openai_api_key" => config.openai.api_key = Some(value.to_string()),
        "openai_model" => config.openai.model = Some(value.to_string()),

        // Ollama settings
        "ollama_model" => config.ollama.model = Some(value.to_string()),
        "ollama_host" => config.ollama.host = Some(value.to_string()),
        "ollama_port" => config.ollama.port = value.parse().ok(),

        _ => log::warn!("Unknown config key: {}", key)
      }
    }
  }

  Ok(config)
}

fn save_config(config: &AppConfig) -> Result<()> {
  if let Some(parent) = CONFIG_PATH.parent() {
    std::fs::create_dir_all(parent).context("Failed to create config directory")?;
  }

  // Convert config to a simple key-value format
  let mut contents = String::new();

  // General settings
  if let Some(provider) = &config.provider {
    contents.push_str(&format!("provider = {}\n", provider));
  }
  if let Some(max_tokens) = config.max_tokens {
    contents.push_str(&format!("max_tokens = {}\n", max_tokens));
  }
  if let Some(max_commit_length) = config.max_commit_length {
    contents.push_str(&format!("max_commit_length = {}\n", max_commit_length));
  }

  // OpenAI settings
  if let Some(api_key) = &config.openai.api_key {
    contents.push_str(&format!("openai_api_key = {}\n", api_key));
  }
  if let Some(model) = &config.openai.model {
    contents.push_str(&format!("openai_model = {}\n", model));
  }

  // Ollama settings
  if let Some(model) = &config.ollama.model {
    contents.push_str(&format!("ollama_model = {}\n", model));
  }
  if let Some(host) = &config.ollama.host {
    contents.push_str(&format!("ollama_host = {}\n", host));
  }
  if let Some(port) = config.ollama.port {
    contents.push_str(&format!("ollama_port = {}\n", port));
  }

  std::fs::write(&*CONFIG_PATH, contents).context("Failed to write config file")?;

  println!("{} Configuration updated!", Emoji("✨", ":-)"));
  Ok(())
}
