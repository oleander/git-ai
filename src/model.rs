use std::default::Default;
use std::fmt::{self, Display};
use std::str::FromStr;
use std::sync::Mutex;
use std::collections::HashMap;

use once_cell::sync::Lazy;
use anyhow::{bail, Result};
use tiktoken_rs::get_completion_max_tokens;
use tiktoken_rs::model::get_context_size;

use crate::profile;

// Token count cache
static TOKEN_CACHE: Lazy<Mutex<HashMap<String, usize>>> = Lazy::new(|| Mutex::new(HashMap::with_capacity(1000)));

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
const MODEL_SLYOTIS: &str = "SlyOtis/git-auto-message:latest";

/// Represents the available AI models for commit message generation.
/// Each model has different capabilities and token limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
  Tavernari,
  /// SlyOtis Git Auto Message model
  SlyOtis
}

impl Model {
  /// Counts the number of tokens in the given text for the current model.
  /// Uses caching to avoid recounting the same text multiple times.
  pub fn count_tokens(&self, text: &str) -> Result<usize> {
    profile!("Count tokens");

    // For very short texts, don't bother with caching
    if text.len() < 50 {
      return self.count_tokens_internal(text);
    }

    let cache_key = format!("{}:{}", self.to_string(), xxhash_rust::xxh3::xxh3_64(text.as_bytes()));

    if let Some(count) = TOKEN_CACHE.lock().unwrap().get(&cache_key) {
      return Ok(*count);
    }

    let count = self.count_tokens_internal(text)?;
    TOKEN_CACHE.lock().unwrap().insert(cache_key, count);

    Ok(count)
  }

  /// Internal method to count tokens without caching
  fn count_tokens_internal(&self, text: &str) -> Result<usize> {
    match self {
      Model::Llama2 | Model::CodeLlama | Model::Mistral | Model::DeepSeekR1_7B | Model::SmollM2 | Model::Tavernari | Model::SlyOtis => {
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
      Model::Llama2 | Model::CodeLlama | Model::Mistral | Model::DeepSeekR1_7B | Model::SmollM2 | Model::Tavernari | Model::SlyOtis =>
        4096_usize,
      _ => {
        let model_str: &str = self.into();
        get_context_size(model_str)
      }
    }
  }

  /// Truncates the given text to fit within the specified token limit.
  pub fn truncate(&self, text: &str, max_tokens: usize) -> Result<String> {
    profile!("Truncate text");

    // For small texts, just count directly
    if let Ok(count) = self.count_tokens(text) {
      if count <= max_tokens {
        return Ok(text.to_string());
      }
    }

    // Process text in larger chunks for efficiency
    let chunk_size = (max_tokens as f64 * 1.5) as usize; // Estimate chunk size
    let mut result = String::with_capacity(text.len());
    let mut current_tokens = 0;

    for chunk in text.lines().collect::<Vec<_>>().chunks(20) {
      let chunk_text = chunk.join("\n");
      let chunk_tokens = self.count_tokens(&chunk_text)?;

      if current_tokens + chunk_tokens <= max_tokens {
        if !result.is_empty() {
          result.push('\n');
        }
        result.push_str(&chunk_text);
        current_tokens += chunk_tokens;
      } else {
        // Process remaining lines individually if close to limit
        for line in chunk {
          let line_tokens = self.count_tokens(line)?;
          if current_tokens + line_tokens > max_tokens {
            break;
          }
          if !result.is_empty() {
            result.push('\n');
          }
          result.push_str(line);
          current_tokens += line_tokens;
        }
        break;
      }
    }

    Ok(result)
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
      Model::Tavernari => MODEL_TAVERNARI,
      Model::SlyOtis => MODEL_SLYOTIS
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
      s if s.eq_ignore_ascii_case(MODEL_SLYOTIS) => Ok(Model::SlyOtis),
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
