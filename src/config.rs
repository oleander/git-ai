use std::io::Write;
use std::path::PathBuf;
use std::fs::File;

use serde::{Deserialize, Serialize};
use config::{Config, FileFormat};
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use console::Emoji;
use thiserror::Error;
use url::Url;

// Constants
const DEFAULT_TIMEOUT: i64 = 30;
const DEFAULT_MAX_COMMIT_LENGTH: i64 = 72;
const DEFAULT_MAX_TOKENS: i64 = 2024;
const DEFAULT_MODEL: &str = "gpt-4o-mini";
const DEFAULT_API_KEY: &str = "<PLACE HOLDER FOR YOUR API KEY>";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub struct App {
  #[serde(flatten)]
  pub app:    AppConfig,
  #[serde(default)]
  pub openai: OpenAI,
  #[serde(default)]
  pub ollama: Ollama,
  #[serde(default)]
  pub git:    Git
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub model:             Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_tokens:        Option<usize>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_commit_length: Option<usize>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub timeout:           Option<u64>
}

impl App {
  pub fn new() -> Result<Self> {
    PATHS.ensure_exists()?;

    let config = Config::builder()
      .add_source(config::Environment::with_prefix("APP").try_parsing(true))
      .add_source(config::File::new(PATHS.file.to_string_lossy().as_ref(), FileFormat::Ini))
      .set_default("app.language", "en")?
      .set_default("app.timeout", DEFAULT_TIMEOUT)?
      .set_default("app.max_commit_length", DEFAULT_MAX_COMMIT_LENGTH)?
      .set_default("app.max_tokens", DEFAULT_MAX_TOKENS)?
      .set_default("app.model", DEFAULT_MODEL)?
      .set_default("openai.key", DEFAULT_API_KEY)?
      .build()?;

    config
      .try_deserialize()
      .context("Failed to deserialize existing config. Please run `git ai config reset` and try again")
  }

  pub fn save(&self) -> Result<()> {
    let contents = serde_ini::to_string(&self).context(format!("Failed to serialize config: {:?}", self))?;
    let mut file = File::create(&PATHS.file).with_context(|| format!("Failed to create config file at {:?}", PATHS.file))?;
    file
      .write_all(contents.as_bytes())
      .context("Failed to write config file")
  }

  pub fn update_model(&mut self, value: String) -> Result<()> {
    self.app.model = Some(value);
    self.save_with_message("model")
  }

  pub fn update_max_tokens(&mut self, value: usize) -> Result<()> {
    self.app.max_tokens = Some(value);
    self.save_with_message("max-tokens")
  }

  pub fn update_max_commit_length(&mut self, value: usize) -> Result<()> {
    self.app.max_commit_length = Some(value);
    self.save_with_message("max-commit-length")
  }

  pub fn update_openai_api_key(&mut self, value: String) -> Result<()> {
    self.openai.key = value;
    self.save_with_message("openai-api-key")
  }

  pub fn update_openai_host(&mut self, value: String) -> Result<()> {
    // Validate URL format
    Url::parse(&value).map_err(|_| ConfigError::InvalidUrl(value.clone()))?;
    self.openai.host = value;
    self.save_with_message("openai-host")
  }

  fn save_with_message(&self, option: &str) -> Result<()> {
    println!("{} Configuration option {} updated!", Emoji("✨", ":-)"), option);
    self.save()
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAI {
  pub host:    String,
  pub key:     String,
  pub model:   String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub api_key: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Git {
  #[serde(with = "bool_as_string")]
  pub enabled: bool
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Ollama {
  #[serde(with = "bool_as_string")]
  pub enabled: bool,
  pub host:    String
}

#[derive(Debug)]
pub struct ConfigPaths {
  pub dir:  PathBuf,
  pub file: PathBuf
}

lazy_static! {
  static ref PATHS: ConfigPaths = ConfigPaths::new();
  pub static ref APP: App = App::new().expect("Failed to load config");
}

impl ConfigPaths {
  fn new() -> Self {
    let dir = home::home_dir()
      .expect("Failed to determine home directory")
      .join(".config/git-ai");
    let file = dir.join("config.ini");
    Self { dir, file }
  }

  fn ensure_exists(&self) -> Result<()> {
    if !self.dir.exists() {
      std::fs::create_dir_all(&self.dir).with_context(|| format!("Failed to create config directory at {:?}", self.dir))?;
    }
    if !self.file.exists() {
      File::create(&self.file).with_context(|| format!("Failed to create config file at {:?}", self.file))?;
    }
    Ok(())
  }
}

#[derive(Error, Debug)]
pub enum ConfigError {
  #[error("Invalid URL format: {0}. The URL should be in the format 'https://api.example.com'")]
  InvalidUrl(String)
}

// Custom serializer for boolean values
mod bool_as_string {
  use serde::{self, Deserialize, Deserializer, Serializer};

  pub fn serialize<S>(value: &bool, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer
  {
    serializer.serialize_str(if *value {
      "1"
    } else {
      "0"
    })
  }

  pub fn deserialize<'de, D>(deserializer: D) -> Result<bool, D::Error>
  where
    D: Deserializer<'de>
  {
    let s = String::deserialize(deserializer)?;
    match s.as_str() {
      "1" | "true" | "yes" | "on" => Ok(true),
      "0" | "false" | "no" | "off" => Ok(false),
      _ => Ok(false) // Default to false for any other value
    }
  }
}
