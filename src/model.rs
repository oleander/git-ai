use std::default::Default;
use std::fmt::{self, Display};
use std::str::FromStr;
use std::sync::Mutex;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

use once_cell::sync::Lazy;
use anyhow::{bail, Result};
use tiktoken_rs::get_completion_max_tokens;
use tiktoken_rs::model::get_context_size;
use rayon::prelude::*;

use crate::profile;

// Token count cache using hash for keys
static TOKEN_CACHE: Lazy<Mutex<HashMap<u64, usize>>> = Lazy::new(|| Mutex::new(HashMap::with_capacity(1000)));

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
  /// Batch counts tokens for multiple texts
  pub fn count_tokens_batch(&self, texts: &[&str]) -> Result<Vec<usize>> {
    if texts.is_empty() {
      return Ok(Vec::new());
    }

    match self {
      Model::Llama2 | Model::CodeLlama | Model::Mistral | Model::DeepSeekR1_7B | Model::SmollM2 | Model::Tavernari | Model::SlyOtis => {
        // Fast path for Ollama models - process in parallel
        Ok(
          texts
            .par_iter()
            .map(|text| self.estimate_tokens(text))
            .collect()
        )
      }
      _ => {
        // For other models, use parallel processing with caching
        let cache = TOKEN_CACHE.lock().unwrap();
        let mut results = vec![0; texts.len()];
        let mut uncached_indices = Vec::new();
        let mut uncached_texts = Vec::new();

        // Check cache for all texts first
        for (i, &text) in texts.iter().enumerate() {
          let cache_key = {
            let mut hasher = DefaultHasher::new();
            self.to_string().hash(&mut hasher);
            text.hash(&mut hasher);
            hasher.finish()
          };

          if let Some(&count) = cache.get(&cache_key) {
            results[i] = count;
          } else {
            uncached_indices.push(i);
            uncached_texts.push((text, cache_key));
          }
        }
        drop(cache); // Release lock before parallel processing

        if !uncached_texts.is_empty() {
          // Process uncached texts in parallel
          let new_counts: Vec<_> = uncached_texts
            .par_iter()
            .map(|(text, cache_key)| {
              let count = self.count_tokens_internal(text)?;
              Ok((*cache_key, count))
            })
            .collect::<Result<Vec<_>>>()?;

          // Update cache with new values in batch
          let mut cache = TOKEN_CACHE.lock().unwrap();
          for (cache_key, count) in &new_counts {
            cache.insert(*cache_key, *count);
          }
          drop(cache);

          // Fill in uncached results
          for (i, (_, count)) in uncached_indices.into_iter().zip(new_counts.iter()) {
            results[i] = *count;
          }
        }

        Ok(results)
      }
    }
  }

  /// Counts tokens for different model types
  #[inline]
  fn estimate_tokens(&self, text: &str) -> usize {
    // Handle empty string case first
    if text.is_empty() {
      return 0;
    }

    match self {
      // For OpenAI models, use tiktoken directly for accurate counts
      Model::GPT4 | Model::GPT4o | Model::GPT4Turbo | Model::GPT4oMini => {
        let model_str: &str = self.into();
        tiktoken_rs::get_completion_max_tokens(model_str, text).unwrap_or_else(|_| self.fallback_estimate(text))
      }

      // For other models, use model-specific heuristics
      Model::Llama2 | Model::CodeLlama | Model::Mistral | Model::DeepSeekR1_7B => {
        // These models use byte-pair encoding with typical ratio of ~0.4 tokens/byte
        self.bpe_estimate(text)
      }

      // For specialized models, use custom ratios based on their tokenization
      Model::SmollM2 => {
        // Smaller models tend to have larger token/byte ratios
        self.bpe_estimate(text) + (text.len() / 10)
      }

      // For commit message models, bias towards natural language tokenization
      Model::Tavernari | Model::SlyOtis => {
        let len = text.len();
        if len > 500 {
          // For very long texts, ensure we meet minimum token requirements
          // Use character count as base and apply scaling
          let char_based = (len as f64 * 0.2).ceil() as usize;
          let nl_based = self.natural_language_estimate(text);
          // Take the maximum of character-based and natural language estimates
          char_based.max(nl_based)
        } else {
          self.natural_language_estimate(text)
        }
      }
    }
  }

  /// BPE-based token estimation
  #[inline]
  fn bpe_estimate(&self, text: &str) -> usize {
    let byte_len = text.len();
    // Ensure at least 1 token for non-empty input
    if byte_len > 0 {
      // Account for UTF-8 characters and common subword patterns
      let utf8_overhead = text.chars().filter(|c| *c as u32 > 127).count() / 2;
      ((byte_len + utf8_overhead) as f64 * 0.4).max(1.0) as usize
    } else {
      0
    }
  }

  /// Natural language token estimation
  #[inline]
  fn natural_language_estimate(&self, text: &str) -> usize {
    if text.is_empty() {
      return 0;
    }

    // Count words and special tokens
    let words = text.split_whitespace().count().max(1); // At least 1 token for non-empty
    let special_chars = text.chars().filter(|c| !c.is_alphanumeric()).count();

    // Check for code blocks and apply higher weight
    let code_block_weight = if text.contains("```") {
      // Count lines within code blocks for better estimation
      let code_lines = text
        .split("```")
        .skip(1) // Skip text before first code block
        .take(1) // Take just the first code block
        .next()
        .map(|block| block.lines().count())
        .unwrap_or(0);

      // Apply higher weight for code blocks:
      // - Base weight for the block markers (6 tokens for ```language and ```)
      // - Each line gets extra weight for syntax
      // - Additional weight for code-specific tokens
      6 + (code_lines * 4)
        + text
          .matches(|c: char| ['{', '}', '(', ')', ';'].contains(&c))
          .count()
          * 2
    } else {
      0
    };

    // Add extra weight for newlines to better handle commit message format
    let newline_weight = text.matches('\n').count();

    words + (special_chars / 2) + code_block_weight + newline_weight
  }

  /// Fallback estimation when tiktoken fails
  #[inline]
  fn fallback_estimate(&self, text: &str) -> usize {
    if text.is_empty() {
      return 0;
    }

    // More conservative estimate for fallback
    let words = text.split_whitespace().count().max(1); // At least 1 token for non-empty
    let chars = text.chars().count();
    let special_chars = text.chars().filter(|c| !c.is_alphanumeric()).count();

    // For very long texts, use a more aggressive scaling factor
    let length_factor = if chars > 500 {
      1.5
    } else {
      1.3
    };

    ((words as f64 * length_factor) + (chars as f64 * 0.1) + (special_chars as f64 * 0.2)).max(1.0) as usize
  }

  /// Counts the number of tokens in the given text
  pub fn count_tokens(&self, text: &str) -> Result<usize> {
    profile!("Count tokens");

    // For very short texts or Ollama models, use fast path
    if text.len() < 50
      || matches!(
        self,
        Model::Llama2 | Model::CodeLlama | Model::Mistral | Model::DeepSeekR1_7B | Model::SmollM2 | Model::Tavernari | Model::SlyOtis
      )
    {
      return Ok(self.estimate_tokens(text));
    }

    // Use a faster hash for caching
    let cache_key = {
      let mut hasher = DefaultHasher::new();
      self.to_string().hash(&mut hasher);
      text.hash(&mut hasher);
      hasher.finish()
    };

    // Fast cache lookup with minimal locking
    {
      let cache = TOKEN_CACHE.lock().unwrap();
      if let Some(&count) = cache.get(&cache_key) {
        return Ok(count);
      }
    }

    let count = self.count_tokens_internal(text)?;

    // Only cache if text is long enough to be worth it
    if text.len() > 100 {
      TOKEN_CACHE.lock().unwrap().insert(cache_key, count);
    }

    Ok(count)
  }

  /// Internal method to count tokens without caching
  fn count_tokens_internal(&self, text: &str) -> Result<usize> {
    let model_str: &str = self.into();
    Ok(
      self
        .context_size()
        .saturating_sub(get_completion_max_tokens(model_str, text)?)
    )
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

    // Check if text already fits within token limit
    if self.count_tokens(text)? <= max_tokens {
      return Ok(text.to_string());
    }

    // Use binary search to find the optimal truncation point
    let lines: Vec<_> = text.lines().collect();
    let total_lines = lines.len();

    // Use exponential search to find a rough cut point
    let mut size = 1;
    while size < total_lines && self.count_tokens(&lines[..size].join("\n"))? <= max_tokens {
      size *= 2;
    }

    // Binary search within the found range
    let mut left = size / 2;
    let mut right = size.min(total_lines);

    while left < right {
      let mid = (left + right).div_ceil(2);
      let chunk = lines[..mid].join("\n");

      if self.count_tokens(&chunk)? <= max_tokens {
        left = mid;
      } else {
        right = mid - 1;
      }
    }

    Ok(lines[..left].join("\n"))
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_gpt4_token_estimation() {
    let model = Model::GPT4;
    // Test with known GPT-4 tokenization patterns
    assert!(model.estimate_tokens("Hello world") >= 2); // Should be at least 2 tokens
    assert!(model.estimate_tokens("Hello, world! 🌎") >= 4); // Should account for special chars and emoji

    // Test longer text with typical patterns
    let code = "fn main() { println!(\"Hello, world!\"); }";
    let estimated = model.estimate_tokens(code);
    assert!(estimated >= 10, "Code snippet estimation too low: {}", estimated);
  }

  #[test]
  fn test_llama_token_estimation() {
    let model = Model::Llama2;
    // Test BPE-based estimation
    let text = "This is a test of the BPE tokenizer";
    let estimated = model.estimate_tokens(text);
    assert!(estimated >= 7, "Basic text estimation too low: {}", estimated);

    // Test with UTF-8 characters
    let text_utf8 = "Hello 世界! こんにちは";
    let basic = model.estimate_tokens("Hello world!");
    let utf8 = model.estimate_tokens(text_utf8);
    assert!(utf8 > basic, "UTF-8 text should have higher token count");
  }

  #[test]
  fn test_commit_message_estimation() {
    let model = Model::Tavernari;
    // Test typical commit message
    let msg = "fix(api): resolve null pointer in user auth\n\nThis commit fixes a null pointer exception that occurred when processing user authentication with empty tokens.";
    let estimated = model.estimate_tokens(msg);
    assert!(estimated >= 20, "Commit message estimation too low: {}", estimated);

    // Test with code snippet in commit message
    let msg_with_code = "fix: Update user model\n\n```rust\nuser.name = name.trim().to_string();\n```";
    let estimated_with_code = model.estimate_tokens(msg_with_code);
    assert!(estimated_with_code > estimated, "Message with code should have higher count");
  }

  #[test]
  fn test_small_model_estimation() {
    let model = Model::SmollM2;
    // Test basic text
    let text = "Testing the small model tokenization";
    let estimated = model.estimate_tokens(text);
    assert!(estimated >= 5, "Basic text estimation too low: {}", estimated);

    // Compare with larger model to verify higher token count
    let large_model = Model::Llama2;
    assert!(
      model.estimate_tokens(text) > large_model.estimate_tokens(text),
      "Small model should estimate more tokens than larger models"
    );
  }

  #[test]
  fn test_edge_cases() {
    let models = [Model::GPT4, Model::Llama2, Model::Tavernari, Model::SmollM2];

    for model in models {
      // Empty string
      assert_eq!(model.estimate_tokens(""), 0, "Empty string should have 0 tokens");

      // Single character
      assert!(model.estimate_tokens("a") > 0, "Single char should have >0 tokens");

      // Whitespace only
      assert!(model.estimate_tokens("   \n   ") > 0, "Whitespace should have >0 tokens");

      // Very long text
      let long_text = "a".repeat(1000);
      assert!(
        model.estimate_tokens(&long_text) >= 100,
        "Long text estimation suspiciously low for {model:?}"
      );
    }
  }

  #[test]
  fn test_fallback_estimation() {
    let model = Model::GPT4;
    // Force fallback by using invalid model string
    let text = "Testing fallback estimation logic";
    let fallback = model.fallback_estimate(text);
    assert!(fallback > 0, "Fallback estimation should be positive");
    assert!(
      fallback >= text.split_whitespace().count(),
      "Fallback should estimate at least one token per word"
    );
  }
}
