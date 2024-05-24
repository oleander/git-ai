use std::default::Default;
use std::str::FromStr;
use std::fmt::{self, Display};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tiktoken_rs::get_completion_max_tokens;
use tiktoken_rs::model::get_context_size;

const GPT4: &str = "gpt-4";
const GPT4O: &str = "gpt-4o";
const GPT4_TURBO: &str = "gpt-4-turbo-preview";

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum Model {
  GPT4,
  GPT4o,
  GPT4Turbo
}

impl Model {
  pub fn count_tokens(&self, text: &str) -> Result<usize> {
    get_completion_max_tokens(self.into(), text)
  }

  pub fn context_size(&self) -> usize {
    get_context_size(self.into())
  }
}

impl From<&Model> for &str {
  fn from(model: &Model) -> Self {
    match model {
      Model::GPT4o => GPT4O,
      Model::GPT4 => GPT4,
      Model::GPT4Turbo => GPT4_TURBO
    }
  }
}

impl Default for Model {
  fn default() -> Self {
    Model::GPT4o
  }
}

impl FromStr for Model {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Self> {
    match s.trim().to_lowercase().as_str() {
      GPT4O => Ok(Model::GPT4o),
      GPT4 => Ok(Model::GPT4),
      GPT4_TURBO => Ok(Model::GPT4Turbo),
      model => bail!("Invalid model: {}", model)
    }
  }
}

impl Display for Model {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", <&str>::from(self))
  }
}

impl From<&str> for Model {
  fn from(s: &str) -> Self {
    s.parse().unwrap_or_default()
  }
}

impl From<String> for Model {
  fn from(s: String) -> Self {
    s.as_str().into()
  }
}
