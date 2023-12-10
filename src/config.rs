use std::io::Write;
use std::path::PathBuf;
use std::fs::File;

use serde::{Deserialize, Serialize};
use config::{Config, FileFormat};
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use console::{style, Emoji};
use clap::ArgMatches;

#[derive(Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct App {
  pub openai_api_key:  Option<String>,
  pub model:           String,
  pub language:        String,
  pub max_diff_tokens: usize,
  pub max_length:      usize,
  pub timeout:         usize
}

impl App {
  #[allow(dead_code)]
  pub fn duration(&self) -> std::time::Duration {
    std::time::Duration::from_secs(self.timeout as u64)
  }
}

lazy_static! {
  pub static ref CONFIG_DIR: PathBuf = home::home_dir().unwrap().join(".config/git-ai");
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
      .set_default("timeout", 30)?
      .set_default("max_length", 72)?
      .set_default("max_diff_tokens", 5000)?
      .set_default("model", "gpt-4-1106-preview")?
      .build()?;

    config.try_deserialize().context("Failed to deserialize config")
  }

  pub fn save(&self) -> Result<()> {
    let contents = serde_ini::to_string(&self).context("Failed to serialize config")?;
    let mut file = File::create(CONFIG_PATH.to_str().unwrap()).context("Failed to create config file")?;
    file.write_all(contents.as_bytes()).context("Failed to write config file")
  }
}

pub fn run(args: &ArgMatches) -> Result<()> {
  let mut app = App::new()?;
  match args.subcommand() {
    Some(("timeout", args)) => {
      app.timeout = *args.get_one("timeout").context("Failed to parse timeout")?;
    },
    Some(("model", args)) => {
      app.model = args.get_one::<String>("<VALUE>").context("Failed to parse model")?.clone();
    },
    Some(("language", args)) => {
      app.language = args.get_one::<String>("<VALUE>").context("Failed to parse language")?.clone();
    },
    Some(("max-diff-tokens", args)) => {
      app.max_diff_tokens = *args.get_one("max-diff-tokens").context("Failed to parse max-diff-tokens")?;
    },
    Some(("max-length", args)) => {
      app.max_length = *args.get_one("max-length").context("Failed to parse max-length")?;
    },
    Some(("openai-api-key", args)) => {
      app.openai_api_key = args
        .get_one::<String>("<VALUE>")
        .context("Failed to parse openai-api-key")?
        .clone()
        .into();
    },
    _ => unreachable!()
  }

  if let Some(key) = args.subcommand_name() {
    println!("{} Configuration option {} updated!", Emoji("✨", ":-)"), style(key).italic());
  }

  app.save()
}
