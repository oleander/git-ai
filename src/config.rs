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
const DEFAULT_OPENAI_HOST: &str = "https://api.openai.com/v1";

#[derive(Debug, Serialize, Deserialize)]
pub struct App {
  pub model:             Option<String>,
  pub max_tokens:        Option<usize>,
  pub max_commit_length: Option<usize>,
  pub timeout:           Option<u64>,
  #[serde(default)]
  pub openai:            OpenAI
}

impl Default for App {
  fn default() -> Self {
    Self {
      model:             None,
      max_tokens:        None,
      max_commit_length: None,
      timeout:           None,
      openai:            OpenAI::default()
    }
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAI {
  #[serde(default = "default_openai_host")]
  pub host:    String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub api_key: Option<String>
}

impl Default for OpenAI {
  fn default() -> Self {
    Self { host: default_openai_host(), api_key: None }
  }
}

fn default_openai_host() -> String {
  std::env::var("OPENAI_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string())
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
  InvalidUrl(String),
  #[error("Failed to save configuration: {0}")]
  SaveError(String)
}

impl App {
  pub fn new() -> Result<Self> {
    dotenv::dotenv().ok();
    PATHS.ensure_exists()?;

    let config = Config::builder()
      .add_source(config::Environment::with_prefix("APP").try_parsing(true))
      .add_source(config::File::new(PATHS.file.to_string_lossy().as_ref(), FileFormat::Ini))
      .set_default("language", "en")?
      .set_default("timeout", DEFAULT_TIMEOUT)?
      .set_default("max_commit_length", DEFAULT_MAX_COMMIT_LENGTH)?
      .set_default("max_tokens", DEFAULT_MAX_TOKENS)?
      .set_default("model", DEFAULT_MODEL)?
      .set_default("openai_api_key", DEFAULT_API_KEY)?
      .set_default("openai_host", DEFAULT_OPENAI_HOST)?
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
    self.model = Some(value);
    self.save_with_message("model")
  }

  pub fn update_max_tokens(&mut self, value: usize) -> Result<()> {
    self.max_tokens = Some(value);
    self.save_with_message("max-tokens")
  }

  pub fn update_max_commit_length(&mut self, value: usize) -> Result<()> {
    self.max_commit_length = Some(value);
    self.save_with_message("max-commit-length")
  }

  pub fn update_openai_api_key(&mut self, value: String) -> Result<()> {
    self.openai.api_key = Some(value);
    self.save_with_message("openai-api-key")
  }

  pub fn update_openai_host(&mut self, value: String) -> Result<()> {
    // Validate URL format
    Url::parse(&value).map_err(|_| ConfigError::InvalidUrl(value.clone()))?;

    self.openai.host = value;
    self
      .save_with_message("openai-host")
      .map_err(|e| ConfigError::SaveError(e.to_string()).into())
  }

  fn save_with_message(&self, option: &str) -> Result<()> {
    println!("{} Configuration option {} updated!", Emoji("✨", ":-)"), option);
    self.save()
  }
}

#[cfg(test)]
mod tests {
  use std::env;

  use super::*;

  #[test]
  fn test_openai_url_configuration() {
    // Test default value
    let app = App::default();
    assert_eq!(app.openai.host, "https://api.openai.com/v1");

    // Test environment variable override
    env::set_var("OPENAI_URL", "https://custom-api.example.com");
    let app = App::default();
    assert_eq!(app.openai.host, "https://custom-api.example.com");
    env::remove_var("OPENAI_URL");

    // Test manual update with valid URL
    let mut app = App::default();
    let test_url = "https://another-api.example.com";
    app.openai.host = test_url.to_string();
    assert_eq!(app.openai.host, test_url);

    // Test URL validation
    let mut app = App::default();
    let result = app.update_openai_host("not-a-url".to_string());
    assert!(result.is_err());
    if let Err(e) = result {
      assert!(e.to_string().contains("Invalid URL format"));
    }
  }
}
