use std::default::Default;
use std::fmt::{self, Display};
use std::str::FromStr;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tiktoken_rs::get_completion_max_tokens;
use tiktoken_rs::model::get_context_size;

use crate::profile;

// Model identifiers - using screaming case for constants
const MODEL_GPT4: &str = "gpt-4";
const MODEL_GPT4_OPTIMIZED: &str = "gpt-4o";
const MODEL_GPT4_MINI: &str = "gpt-4o-mini";
const MODEL_GPT4_TURBO: &str = "gpt-4-turbo-preview";
const MODEL_LLAMA2: &str = "llama2:latest";
const MODEL_CODELLAMA: &str = "codellama:latest";
const MODEL_MISTRAL: &str = "mistral:latest";
const MODEL_DEEPSEEK: &str = "deepseek-r1:7b";
const MODEL_SMOLLM2: &str = "smollm2:135m";
const MODEL_TAVERNARI: &str = "tavernari/git-commit-message:latest";

/// Represents the available AI models for commit message generation.
/// Each model has different capabilities and token limits.
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize, Default)]
pub enum Model {
  /// Standard GPT-4 model
  GPT4,
  /// Optimized GPT-4 model for better performance
  GPT4o,
  /// GPT-4 Turbo model
  GPT4Turbo,
  /// Default model - Mini version of optimized GPT-4 for faster processing
  #[default]
  GPT4oMini,
  /// Llama 2 model
  Llama2,
  /// CodeLlama model optimized for code
  CodeLlama,
  /// Mistral model
  Mistral,
  /// DeepSeek model
  DeepSeekR1_7B,
  /// Smol LM 2 135M model
  SmollM2,
  /// Tavernari Git Commit Message model
  Tavernari
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
    match self {
      Model::Llama2 | Model::CodeLlama | Model::Mistral | Model::DeepSeekR1_7B | Model::SmollM2 | Model::Tavernari => {
        // For Ollama models, we'll estimate tokens based on word count
        // A rough approximation is that each word is about 1.3 tokens
        let word_count = text.split_whitespace().count();
        Ok((word_count as f64 * 1.3).ceil() as usize)
      }
      _ => {
        let model_str: &str = self.into();
        Ok(
          self
            .context_size()
            .saturating_sub(get_completion_max_tokens(model_str, text)?)
        )
      }
    }
  }

  /// Gets the maximum context size for the current model.
  ///
  /// # Returns
  /// * `usize` - The maximum number of tokens the model can process
  pub fn context_size(&self) -> usize {
    profile!("Get context size");
    match self {
      Model::Llama2 | Model::CodeLlama | Model::Mistral | Model::DeepSeekR1_7B | Model::SmollM2 | Model::Tavernari => 4096_usize,
      _ => {
        let model_str: &str = self.into();
        get_context_size(model_str)
      }
    }
  }

  /// Truncates the given text to fit within the specified token limit.
  ///
  /// # Arguments
  /// * `text` - The text to truncate
  /// * `max_tokens` - The maximum number of tokens allowed
  ///
  /// # Returns
  /// * `Result<String>` - The truncated text or an error
  pub(crate) fn truncate(&self, text: &str, max_tokens: usize) -> Result<String> {
    profile!("Truncate text");
    self.walk_truncate(text, max_tokens, usize::MAX)
  }

  /// Recursively truncates text to fit within token limits while maintaining coherence.
  /// Uses a binary search-like approach to find the optimal truncation point.
  ///
  /// # Arguments
  /// * `text` - The text to truncate
  /// * `max_tokens` - The maximum number of tokens allowed
  /// * `within` - The maximum allowed deviation from target token count
  ///
  /// # Returns
  /// * `Result<String>` - The truncated text or an error
  pub(crate) fn walk_truncate(&self, text: &str, max_tokens: usize, within: usize) -> Result<String> {
    profile!("Walk truncate iteration");
    log::debug!("max_tokens: {}, within: {}", max_tokens, within);

    let truncated = {
      profile!("Split and join text");
      text
        .split_whitespace()
        .take(max_tokens)
        .collect::<Vec<&str>>()
        .join(" ")
    };

    let token_count = self.count_tokens(&truncated)?;
    let offset = token_count.saturating_sub(max_tokens);

    if offset > within || offset == 0 {
      Ok(truncated)
    } else {
      // Recursively adjust token count to get closer to target
      self.walk_truncate(text, max_tokens + offset, within)
    }
  }
}

impl From<&Model> for &str {
  fn from(model: &Model) -> Self {
    match model {
      Model::GPT4o => MODEL_GPT4_OPTIMIZED,
      Model::GPT4 => MODEL_GPT4,
      Model::GPT4Turbo => MODEL_GPT4_TURBO,
      Model::GPT4oMini => MODEL_GPT4_MINI,
      Model::Llama2 => MODEL_LLAMA2,
      Model::CodeLlama => MODEL_CODELLAMA,
      Model::Mistral => MODEL_MISTRAL,
      Model::DeepSeekR1_7B => MODEL_DEEPSEEK,
      Model::SmollM2 => MODEL_SMOLLM2,
      Model::Tavernari => MODEL_TAVERNARI
    }
  }
}

impl FromStr for Model {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Self> {
    match s.trim().to_lowercase().as_str() {
      s if s.eq_ignore_ascii_case(MODEL_GPT4_OPTIMIZED) => Ok(Model::GPT4o),
      s if s.eq_ignore_ascii_case(MODEL_GPT4) => Ok(Model::GPT4),
      s if s.eq_ignore_ascii_case(MODEL_GPT4_TURBO) => Ok(Model::GPT4Turbo),
      s if s.eq_ignore_ascii_case(MODEL_GPT4_MINI) => Ok(Model::GPT4oMini),
      s if s.eq_ignore_ascii_case(MODEL_LLAMA2) => Ok(Model::Llama2),
      s if s.eq_ignore_ascii_case(MODEL_CODELLAMA) => Ok(Model::CodeLlama),
      s if s.eq_ignore_ascii_case(MODEL_MISTRAL) => Ok(Model::Mistral),
      s if s.eq_ignore_ascii_case(MODEL_DEEPSEEK) => Ok(Model::DeepSeekR1_7B),
      s if s.eq_ignore_ascii_case(MODEL_SMOLLM2) => Ok(Model::SmollM2),
      s if s.eq_ignore_ascii_case(MODEL_TAVERNARI) => Ok(Model::Tavernari),
      model => bail!("Invalid model name: {}", model)
    }
  }
}

impl Display for Model {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", <&str>::from(self))
  }
}

// Implement conversion from string types to Model with fallback to default
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
