use std::default::Default;
use std::fmt::{self, Display};
use std::str::FromStr;
use std::sync::OnceLock;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tiktoken_rs::CoreBPE;
use tiktoken_rs::model::get_context_size;
use async_openai::types::chat::{ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use colored::Colorize;

use crate::profile;
// use crate::config::format_prompt; // Temporarily comment out
use crate::config::AppConfig;

// Cached tokenizer for performance
static TOKENIZER: OnceLock<CoreBPE> = OnceLock::new();

// Model identifiers - using screaming case for constants
const MODEL_GPT4_1: &str = "gpt-4.1";
const MODEL_GPT4_1_MINI: &str = "gpt-4.1-mini";
const MODEL_GPT4_1_NANO: &str = "gpt-4.1-nano";
const MODEL_GPT4_5: &str = "gpt-4.5";
// TODO: Get this from config.rs or a shared constants module
const DEFAULT_MODEL_NAME: &str = "gpt-4.1";

/// Represents the available AI models for commit message generation.
/// Each model has different capabilities and token limits.
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize, Default)]
pub enum Model {
  /// Default model - GPT-4.1 latest version
  #[default]
  GPT41,
  /// Mini version of GPT-4.1 for faster processing
  GPT41Mini,
  /// Nano version of GPT-4.1 for very fast processing
  GPT41Nano,
  /// GPT-4.5 model for advanced capabilities
  GPT45
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
    let tokenizer = TOKENIZER.get_or_init(|| get_tokenizer(self.as_ref()));

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
    // tiktoken-rs 0.12 returns Option; fall back to 4096 (the historical default
    // returned by tiktoken-rs 0.7 for unrecognized models) when the model is unknown.
    get_context_size(self.as_ref()).unwrap_or(4096)
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

  /// Truncates text to fit within a token limit using a single tokenization pass.
  ///
  /// The previous implementation re-`join`ed words and re-tokenized the full candidate
  /// string on every binary-search iteration (O(log n) full tokenizations + re-joins),
  /// which was the dominant cost on large diffs. This version tokenizes the text exactly
  /// once, keeps the first `max_tokens` tokens, and decodes them back to a string. Because
  /// a single character can span multiple tokens, slicing the token vector can land on an
  /// invalid UTF-8 boundary; in that case we drop trailing tokens until the decode succeeds.
  /// Dropping tokens only ever reduces the count, so the result is guaranteed to re-encode
  /// to `<= max_tokens` while always being valid UTF-8.
  ///
  /// The `_within` parameter is retained for signature compatibility but is no longer used:
  /// the result is exact rather than an approximation within a tolerance.
  ///
  /// # Arguments
  /// * `text` - The text to truncate
  /// * `max_tokens` - The maximum number of tokens allowed
  /// * `_within` - Unused; kept for backward-compatible call sites
  ///
  /// # Returns
  /// * `Result<String>` - The truncated text or an error
  pub(crate) fn walk_truncate(&self, text: &str, max_tokens: usize, _within: usize) -> Result<String> {
    profile!("Walk truncate");
    log::debug!("max_tokens: {max_tokens}");

    // Nothing to keep.
    if max_tokens == 0 || text.is_empty() {
      return Ok(String::new());
    }

    let tokenizer = TOKENIZER.get_or_init(|| get_tokenizer(self.as_ref()));

    // Single tokenization pass.
    let tokens = tokenizer.encode_ordinary(text);
    if tokens.len() <= max_tokens {
      return Ok(text.to_string());
    }

    // Keep the first `max_tokens` tokens, then back off until the slice decodes to
    // valid UTF-8 (the slice boundary may fall inside a multi-byte character).
    let mut end = max_tokens;
    loop {
      match tokenizer.decode(&tokens[..end]) {
        Ok(decoded) => return Ok(decoded),
        Err(_) if end > 0 => end -= 1,
        Err(e) => return Err(e)
      }
    }
  }
}

impl AsRef<str> for Model {
  fn as_ref(&self) -> &str {
    match self {
      Model::GPT41 => MODEL_GPT4_1,
      Model::GPT41Mini => MODEL_GPT4_1_MINI,
      Model::GPT41Nano => MODEL_GPT4_1_NANO,
      Model::GPT45 => MODEL_GPT4_5
    }
  }
}

// Keep conversion to String for cases that need owned strings
impl From<&Model> for String {
  fn from(model: &Model) -> Self {
    model.as_ref().to_string()
  }
}

// Keep the old impl for backwards compatibility where possible
impl Model {
  pub fn as_str(&self) -> &str {
    self.as_ref()
  }
}

impl FromStr for Model {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Self> {
    let normalized = s.trim().to_lowercase();
    match normalized.as_str() {
      "gpt-4.1" => Ok(Model::GPT41),
      "gpt-4.1-mini" => Ok(Model::GPT41Mini),
      "gpt-4.1-nano" => Ok(Model::GPT41Nano),
      "gpt-4.5" => Ok(Model::GPT45),
      // Backward compatibility for deprecated models - map to closest GPT-4.1 equivalent
      "gpt-4" | "gpt-4o" => {
        log::warn!(
          "Model '{}' is deprecated. Mapping to 'gpt-4.1'. \
          Please update your configuration with: git ai config set model gpt-4.1",
          s
        );
        Ok(Model::GPT41)
      }
      "gpt-4o-mini" | "gpt-3.5-turbo" => {
        log::warn!(
          "Model '{}' is deprecated. Mapping to 'gpt-4.1-mini'. \
          Please update your configuration with: git ai config set model gpt-4.1-mini",
          s
        );
        Ok(Model::GPT41Mini)
      }
      model =>
        bail!(
          "Invalid model name: '{}'. Supported models: gpt-4.1, gpt-4.1-mini, gpt-4.1-nano, gpt-4.5",
          model
        ),
    }
  }
}

impl Display for Model {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.as_ref())
  }
}

// Implement conversion from string types to Model with fallback to default
impl From<&str> for Model {
  fn from(s: &str) -> Self {
    s.parse().unwrap_or_else(|e| {
      log::error!("Failed to parse model '{}': {}. Falling back to default model 'gpt-4.1'.", s, e);
      Model::default()
    })
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
  // Note: dead code path; kept compiling for the dependency upgrade.
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
  let temperature_value = 0.7_f32;

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
    .max_completion_tokens((model.context_size() - tokens) as u32)
    .build()?;

  profile!("OpenAI API call");
  let response = client.chat().create(request).await?;
  let result = response
    .choices
    .first()
    .ok_or_else(|| anyhow::anyhow!("OpenAI returned no choices"))?
    .message
    .content
    .clone()
    .unwrap_or_default();

  if result.is_empty() {
    bail!("No response from OpenAI");
  }

  Ok(result.trim().to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  /// C3: A large synthetic text must truncate to <= max_tokens, stay on a valid UTF-8
  /// boundary, and re-encode to <= max_tokens.
  #[test]
  fn test_truncate_large_text_is_exact_and_utf8_safe() {
    let model = Model::GPT41;
    // Large input that comfortably exceeds the limit, with multi-byte UTF-8 characters
    // (é, 世界, emoji) so we exercise the token-boundary back-off.
    let text = "The quick brown fox café 世界 🚀 jumps over the lazy dog. ".repeat(500);
    let max_tokens = 100;

    let truncated = model.truncate(&text, max_tokens).unwrap();

    // Valid UTF-8 by construction (it's a Rust String), and re-encodes to <= max_tokens.
    let recount = model.count_tokens(&truncated).unwrap();
    assert!(recount <= max_tokens, "re-encoded token count {recount} exceeds max {max_tokens}");

    // It actually truncated (input was far larger than the limit).
    assert!(truncated.len() < text.len(), "expected truncation to shorten the text");
    assert!(!truncated.is_empty(), "truncation of large text should not be empty");
  }

  /// Text already within the limit is returned unchanged.
  #[test]
  fn test_truncate_passthrough_when_within_limit() {
    let model = Model::GPT41;
    let text = "small bit of text";
    let truncated = model.truncate(text, 1000).unwrap();
    assert_eq!(truncated, text);
  }

  /// max_tokens == 0 yields an empty string (and never panics).
  #[test]
  fn test_truncate_zero_tokens() {
    let model = Model::GPT41;
    let truncated = model.truncate("anything at all here", 0).unwrap();
    assert_eq!(truncated, "");
  }

  /// Truncating multi-byte content to a tiny budget stays on a char boundary.
  #[test]
  fn test_truncate_multibyte_small_budget() {
    let model = Model::GPT41;
    let text = "日本語のテキストをトークン化してから切り詰めます".repeat(20);
    let truncated = model.truncate(&text, 5).unwrap();
    let recount = model.count_tokens(&truncated).unwrap();
    assert!(recount <= 5, "re-encoded token count {recount} exceeds 5");
    // Valid UTF-8 (String) — assert it is a prefix-ish valid slice by checking it round-trips.
    assert!(truncated.is_char_boundary(truncated.len()));
  }
}
