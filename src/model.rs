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

  /// Fast token estimation for Ollama models
  #[inline]
  fn estimate_tokens(&self, text: &str) -> usize {
    // Fast approximation based on byte length
    ((text.len() as f64) * 0.25) as usize
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

    // For small texts or if we're using Ollama, use fast estimation
    if text.len() < 1000
      || matches!(
        self,
        Model::Llama2 | Model::CodeLlama | Model::Mistral | Model::DeepSeekR1_7B | Model::SmollM2 | Model::Tavernari | Model::SlyOtis
      )
    {
      let estimated_tokens = self.estimate_tokens(text);
      if estimated_tokens <= max_tokens {
        return Ok(text.to_string());
      }

      // Estimate how much text we can keep based on the token ratio
      let keep_ratio = max_tokens as f64 / estimated_tokens as f64;
      let keep_bytes = (text.len() as f64 * keep_ratio) as usize;

      // Find the last line break before our estimated cut point
      let result = text.chars().take(keep_bytes).collect::<String>();
      return Ok(result);
    }

    // For other models, use parallel binary search
    let lines: Vec<_> = text.lines().collect();
    let total_lines = lines.len();

    // Use exponential search to find a rough cut point
    let mut size = 1;
    while size < total_lines && self.estimate_tokens(&lines[..size].join("\n")) <= max_tokens {
      size *= 2;
    }

    // Binary search within the found range
    let mut left = size / 2;
    let mut right = size.min(total_lines);

    // Process multiple points in parallel during binary search
    while left < right {
      let mid = (left + right + 1) / 2;
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
