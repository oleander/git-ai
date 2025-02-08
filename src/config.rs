use std::io::Write;
use std::path::PathBuf;
use std::fs::File;

use serde::{Deserialize, Serialize};
use config::{Config, FileFormat};
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use console::Emoji;

// Constants
const DEFAULT_TIMEOUT: i64 = 30;
const DEFAULT_MAX_COMMIT_LENGTH: i64 = 72;
const DEFAULT_MAX_TOKENS: i64 = 2024;
const DEFAULT_MODEL: &str = "gpt-4o-mini";
const DEFAULT_API_KEY: &str = "<PLACE HOLDER FOR YOUR API KEY>";

#[derive(Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct App {
  pub openai_api_key:    Option<String>,
  pub model:             Option<String>,
  pub max_tokens:        Option<usize>,
  pub max_commit_length: Option<usize>,
  pub timeout:           Option<usize>
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
    self.openai_api_key = Some(value);
    self.save_with_message("openai-api-key")
  }

  fn save_with_message(&self, option: &str) -> Result<()> {
    println!("{} Configuration option {} updated!", Emoji("✨", ":-)"), option);
    self.save()
  }
}
