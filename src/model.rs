use std::default::Default;
use std::fmt::{self, Display};
use std::str::FromStr;
use std::sync::OnceLock;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tiktoken_rs::CoreBPE;
use tiktoken_rs::model::get_context_size;
use async_openai::types::{ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use colored::Colorize;

use crate::profile;
// use crate::config::format_prompt; // Temporarily comment out
use crate::config::AppConfig;

// Cached tokenizer for performance
static TOKENIZER: OnceLock<CoreBPE> = OnceLock::new();

// Model identifiers - using screaming case for constants
const MODEL_GPT4: &str = "gpt-4";
const MODEL_GPT4_OPTIMIZED: &str = "gpt-4o";
const MODEL_GPT4_MINI: &str = "gpt-4o-mini";
const MODEL_GPT4_1: &str = "gpt-4.1";
// TODO: Get this from config.rs or a shared constants module
const DEFAULT_MODEL_NAME: &str = "gpt-4.1";

/// Represents the available AI models for commit message generation.
/// Each model has different capabilities and token limits.
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize, Default)]
pub enum Model {
  /// Standard GPT-4 model
  GPT4,
  /// Optimized GPT-4 model for better performance
  GPT4o,
  /// Mini version of optimized GPT-4 for faster processing
  GPT4oMini,
  /// Default model - GPT-4.1 latest version
  #[default]
  GPT41
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

    // Fast path for empty text
    if text.is_empty() {
      return Ok(0);
    }

    // Always use the proper tokenizer for accurate counts
    // We cannot afford to underestimate tokens as it may cause API failures
    let tokenizer = TOKENIZER.get_or_init(|| {
      let model_str: &str = self.into();
      get_tokenizer(model_str)
    });

    // Use direct tokenization for accurate token count
    let tokens = tokenizer.encode_ordinary(text);
    Ok(tokens.len())
  }

  /// Gets the maximum context size for the current model.
  ///
  /// # Returns
  /// * `usize` - The maximum number of tokens the model can process
  pub fn context_size(&self) -> usize {
    profile!("Get context size");
    let model_str: &str = self.into();
    get_context_size(model_str)
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
    log::debug!("max_tokens: {max_tokens}, within: {within}");

    // Check if text already fits within token limit
    let current_tokens = self.count_tokens(text)?;
    if current_tokens <= max_tokens {
      return Ok(text.to_string());
    }

    // Binary search approach to find the right truncation point
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut left = 0;
    let mut right = words.len();
    let mut best_fit = String::new();
    let mut best_tokens = 0;

    // Perform binary search to find optimal word count
    while left < right {
      let mid = (left + right).div_ceil(2);
      let candidate = words[..mid].join(" ");
      let tokens = self.count_tokens(&candidate)?;

      if tokens <= max_tokens {
        // This fits, try to find a longer text that still fits
        best_fit = candidate;
        best_tokens = tokens;
        left = mid;
      } else {
        // Too many tokens, try shorter text
        right = mid - 1;
      }

      // If we're close enough to the target, we can stop
      if best_tokens > 0 && max_tokens.saturating_sub(best_tokens) <= within {
        break;
      }
    }

    // If we couldn't find any fitting text, truncate more aggressively
    if best_fit.is_empty() && !words.is_empty() {
      // Try with just one word
      best_fit = words[0].to_string();
      let tokens = self.count_tokens(&best_fit)?;

      // If even one word is too long, truncate at character level
      if tokens > max_tokens {
        // Estimate character limit based on token limit
        // Conservative estimate: ~3 chars per token
        let char_limit = max_tokens * 3;
        best_fit = text.chars().take(char_limit).collect();

        // Ensure we don't exceed token limit
        while self.count_tokens(&best_fit)? > max_tokens && !best_fit.is_empty() {
          // Remove last 10% of characters
          let new_len = (best_fit.len() * 9) / 10;
          best_fit = best_fit.chars().take(new_len).collect();
        }
      }
    }

    Ok(best_fit)
  }
}

impl From<&Model> for &str {
  fn from(model: &Model) -> Self {
    match model {
      Model::GPT4o => MODEL_GPT4_OPTIMIZED,
      Model::GPT4 => MODEL_GPT4,
      Model::GPT4oMini => MODEL_GPT4_MINI,
      Model::GPT41 => MODEL_GPT4_1
    }
  }
}

impl FromStr for Model {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Self> {
    match s.trim().to_lowercase().as_str() {
      "gpt-4o" => Ok(Model::GPT4o),
      "gpt-4" => Ok(Model::GPT4),
      "gpt-4o-mini" => Ok(Model::GPT4oMini),
      "gpt-4.1" => Ok(Model::GPT41),
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

fn get_tokenizer(_model_str: &str) -> CoreBPE {
  // TODO: This should be based on the model string, but for now we'll just use cl100k_base
  // which is used by gpt-3.5-turbo and gpt-4
  tiktoken_rs::cl100k_base().expect("Failed to create tokenizer")
}

pub async fn run(settings: AppConfig, content: String) -> Result<String> {
  let model_str = settings.model.as_deref().unwrap_or(DEFAULT_MODEL_NAME);

  let client = async_openai::Client::new();
  // let prompt = format_prompt(&content, &settings.prompt(), settings.template())?; // Temporarily comment out
  let prompt = content; // Use raw content as prompt for now
  let model: Model = settings
    .model
    .as_deref()
    .unwrap_or(DEFAULT_MODEL_NAME)
    .into();
  let tokens = model.count_tokens(&prompt)?;

  if tokens > model.context_size() {
    bail!(
      "Input too large: {} tokens. Max {} tokens for {}",
      tokens.to_string().red(),
      model.context_size().to_string().green(),
      model_str.yellow()
    );
  }

  // TODO: Make temperature configurable
  let temperature_value = 0.7;

  log::info!(
    "Using model: {}, Tokens: {}, Max tokens: {}, Temperature: {}",
    model_str.yellow(),
    tokens.to_string().green(),
    // TODO: Make max_tokens configurable
    (model.context_size() - tokens).to_string().green(),
    temperature_value.to_string().blue() // Use temperature_value
  );

  let request = CreateChatCompletionRequestArgs::default()
    .model(model_str)
    .messages([ChatCompletionRequestUserMessageArgs::default()
      .content(prompt)
      .build()?
      .into()])
    .temperature(temperature_value) // Use temperature_value
    // TODO: Make max_tokens configurable
    .max_tokens((model.context_size() - tokens) as u16)
    .build()?;

  profile!("OpenAI API call");
  let response = client.chat().create(request).await?;
  let result = response.choices[0]
    .message
    .content
    .clone()
    .unwrap_or_default();

  if result.is_empty() {
    bail!("No response from OpenAI");
  }

  Ok(result.trim().to_string())
}
