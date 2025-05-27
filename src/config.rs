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
const DEFAULT_MODEL: &str = "gpt-4o";
const DEFAULT_API_KEY: &str = "<PLACE HOLDER FOR YOUR API KEY>";

#[derive(Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct App {
  pub openai_api_key:     Option<String>,
  pub model:              Option<String>,
  pub max_tokens:         Option<usize>,
  pub max_commit_length:  Option<usize>,
  pub timeout:            Option<usize>,
  pub enable_multi_step:  Option<bool>,
  pub parallel_api_calls: Option<bool>
}

impl App {
  #[allow(dead_code)]
  pub fn duration(&self) -> std::time::Duration {
    std::time::Duration::from_secs(self.timeout.unwrap_or(30) as u64)
  }
}

lazy_static! {
    pub static ref CONFIG_DIR: PathBuf = home::home_dir().unwrap().join(".config/git-ai");
    #[derive(Debug)]
    pub static ref APP: App = App::new().expect("Failed to load config");
    pub static ref CONFIG_PATH: PathBuf = CONFIG_DIR.join("config.ini");
}

impl App {
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
      .set_default("timeout", DEFAULT_TIMEOUT)?
      .set_default("max_commit_length", DEFAULT_MAX_COMMIT_LENGTH)?
      .set_default("max_tokens", DEFAULT_MAX_TOKENS)?
      .set_default("model", DEFAULT_MODEL)?
      .set_default("openai_api_key", DEFAULT_API_KEY)?
      .set_default("enable_multi_step", true)?
      .set_default("parallel_api_calls", true)?
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

  pub fn update_enable_multi_step(&mut self, value: bool) -> Result<()> {
    self.enable_multi_step = Some(value);
    self.save_with_message("enable-multi-step")
  }

  pub fn update_parallel_api_calls(&mut self, value: bool) -> Result<()> {
    self.parallel_api_calls = Some(value);
    self.save_with_message("parallel-api-calls")
  }

  fn save_with_message(&self, option: &str) -> Result<()> {
    println!("{} Configuration option {} updated!", Emoji("✨", ":-)"), option);
    self.save()
  }
}

// Standalone functions for backward compatibility
pub fn run_model(value: String) -> Result<()> {
  let mut app = App::new()?;
  app.update_model(value)
}

pub fn run_max_tokens(max_tokens: usize) -> Result<()> {
  let mut app = App::new()?;
  app.update_max_tokens(max_tokens)
}

pub fn run_max_commit_length(max_commit_length: usize) -> Result<()> {
  let mut app = App::new()?;
  app.update_max_commit_length(max_commit_length)
}

pub fn run_openai_api_key(value: String) -> Result<()> {
  let mut app = App::new()?;
  app.update_openai_api_key(value)
}

pub fn run_reset() -> Result<()> {
  if !CONFIG_PATH.exists() {
    eprintln!("{} Configuration file does not exist!", Emoji("🤷", ":-)"));
    return Ok(());
  }

  std::fs::remove_file(CONFIG_PATH.to_str().unwrap()).context("Failed to remove config file")?;
  println!("{} Configuration reset!", Emoji("✨", ":-)"));
  Ok(())
}
