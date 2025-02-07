use std::default::Default;
use std::fmt::{self, Display};
use std::str::FromStr;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tiktoken_rs::get_completion_max_tokens;
use tiktoken_rs::model::get_context_size;

use crate::profile;

// Model identifiers
const GPT4: &str = "gpt-4";
const GPT4O: &str = "gpt-4o";
const GPT4OMINI: &str = "gpt-4o-mini";

/// Represents the available AI models for commit message generation.
/// Each model has different capabilities and token limits.
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize, Default)]
pub enum Model {
  /// Standard GPT-4 model
  GPT4,
  /// Optimized GPT-4 model
  GPT4o,
  /// Default model - Mini version of optimized GPT-4
  #[default]
  GPT4oMini
}

impl Model {
  /// Counts the number of tokens in the given text for the current model.
  /// This is used to ensure we stay within the model's token limits.
  ///
  /// # Arguments
  /// * `text` - The text to count tokens for
  ///
  /// # Returns
  /// * `Result<usize>` - The number of tokens or an error
  pub fn count_tokens(&self, text: &str) -> Result<usize> {
    profile!("Count tokens");
    Ok(
      self
        .context_size()
        .saturating_sub(get_completion_max_tokens(self.into(), text)?)
    )
  }

  /// Gets the maximum context size for the current model.
  ///
  /// # Returns
  /// * `usize` - The maximum number of tokens the model can process
  pub fn context_size(&self) -> usize {
    profile!("Get context size");
    get_context_size(self.into())
  }

  /// Truncates the given text to fit within the specified token limit.
  ///
  /// # Arguments
  /// * `diff` - The text to truncate
  /// * `max_tokens` - The maximum number of tokens allowed
  ///
  /// # Returns
  /// * `Result<String>` - The truncated text or an error
  pub(crate) fn truncate(&self, diff: &str, max_tokens: usize) -> Result<String> {
    profile!("Truncate text");
    self.walk_truncate(diff, max_tokens, usize::MAX)
  }

  /// Recursively truncates text to fit within token limits while maintaining coherence.
  ///
  /// # Arguments
  /// * `diff` - The text to truncate
  /// * `max_tokens` - The maximum number of tokens allowed
  /// * `within` - The maximum allowed deviation from target token count
  ///
  /// # Returns
  /// * `Result<String>` - The truncated text or an error
  pub(crate) fn walk_truncate(&self, diff: &str, max_tokens: usize, within: usize) -> Result<String> {
    profile!("Walk truncate iteration");
    log::debug!("max_tokens: {}", max_tokens);
    log::debug!("diff: {}", diff);
    log::debug!("within: {}", within);

    let str = {
      profile!("Split and join text");
      diff
        .split_whitespace()
        .take(max_tokens)
        .collect::<Vec<&str>>()
        .join(" ")
    };

    let offset = self.count_tokens(&str)?.saturating_sub(max_tokens);

    if offset > within || offset == 0 {
      Ok(str)
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
