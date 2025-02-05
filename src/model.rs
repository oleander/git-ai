use std::default::Default;
use std::fmt::{self, Display};
use std::str::FromStr;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tiktoken_rs::get_completion_max_tokens;
use tiktoken_rs::model::get_context_size;

const GPT4: &str = "gpt-4";
const GPT4O: &str = "gpt-4o";
const GPT4_TURBO: &str = "gpt-4-turbo-preview";
const LLAMA3: &str = "llama3";
const GPT4OMINI: &str = "gpt-4o-mini";

#[derive(Debug, Clone, PartialEq)]
pub struct Response {
  pub response: String
}

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
  pub prompt:     String,
  pub system:     String,
  pub max_tokens: usize,
  pub model:      Model
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize, Default)]
pub enum Model {
  GPT4,
  GPT4o,
  GPT4Turbo,
  Llama3,
  #[default]
  GPT4oMini
}

impl Model {
  pub fn count_tokens(&self, text: &str) -> Result<usize> {
    let max_tokens = get_completion_max_tokens(self.into(), text).unwrap_or(8192);

    Ok(
      self
        .context_size()
        .saturating_sub(max_tokens)
    )
  }

  pub fn context_size(&self) -> usize {
    get_context_size(self.into())
  }

  pub(crate) fn truncate(&self, diff: &str, max_tokens: usize) -> Result<String> {
    self.walk_truncate(diff, max_tokens, usize::MAX)
  }

  pub(crate) fn walk_truncate(&self, diff: &str, max_tokens: usize, within: usize) -> Result<String> {
    log::debug!("max_tokens: {}", max_tokens);
    log::debug!("diff: {}", diff);
    log::debug!("within: {}", within);

    let str = diff
      .split_whitespace()
      .take(max_tokens)
      .collect::<Vec<&str>>()
      .join(" ");
    let offset = self.count_tokens(&str)?.saturating_sub(max_tokens);

    if offset > within || offset == 0 {
      Ok(str) // TODO: check if this is correct
    } else {
      self.walk_truncate(diff, max_tokens + offset, within)
    }
  }
}

impl From<&Model> for &str {
  fn from(model: &Model) -> Self {
    match model {
      Model::GPT4o => GPT4O,
      Model::GPT4 => GPT4,
      Model::GPT4Turbo => GPT4_TURBO,
      Model::Llama3 => LLAMA3,
      Model::GPT4oMini => GPT4OMINI
    }
  }
}

impl FromStr for Model {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Self> {
    match s.trim().to_lowercase().as_str() {
      GPT4O => Ok(Model::GPT4o),
      GPT4 => Ok(Model::GPT4),
      GPT4_TURBO => Ok(Model::GPT4Turbo),
      LLAMA3 => Ok(Model::Llama3),
      GPT4OMINI => Ok(Model::GPT4oMini),
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
