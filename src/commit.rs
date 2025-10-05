use anyhow::{anyhow, bail, Result};
use maplit::hashmap;
use mustache;

use crate::{config, openai, profile};
use crate::model::Model;
use crate::config::AppConfig;

/// The instruction template included at compile time
const INSTRUCTION_TEMPLATE: &str = include_str!("../resources/prompt.md");

/// Returns the instruction template for the AI model.
/// This template guides the model in generating appropriate commit messages.
///
/// # Returns
/// * `Result<String>` - The rendered template or an error
///
/// Note: This function is public only for testing purposes
#[doc(hidden)]
pub fn get_instruction_template() -> Result<String> {
  profile!("Generate instruction template");
  let max_length = config::APP_CONFIG
    .max_commit_length
    .unwrap_or(72)
    .to_string();
  let template = mustache::compile_str(INSTRUCTION_TEMPLATE)
    .map_err(|e| anyhow!("Template compilation error: {}", e))?
    .render_to_string(&hashmap! {
      "max_length" => max_length
    })
    .map_err(|e| anyhow!("Template rendering error: {}", e))?;
  Ok(template)
}

/// Creates an OpenAI request for commit message generation.
///
/// # Arguments
/// * `diff` - The git diff to generate a commit message for
/// * `max_tokens` - Maximum number of tokens allowed for the response
/// * `model` - The AI model to use for generation
///
/// # Returns
/// * `Result<openai::Request>` - The prepared request
///
/// Note: This function is public only for testing purposes
#[doc(hidden)]
pub fn create_commit_request(diff: String, max_tokens: usize, model: Model) -> Result<openai::Request> {
  profile!("Prepare OpenAI request");
  let template = get_instruction_template()?;
  Ok(openai::Request {
    system: template,
    prompt: diff,
    max_tokens: max_tokens.try_into().unwrap_or(u16::MAX),
    model
  })
}

/// Generates a commit message using the AI model.
/// Now uses the multi-step approach by default with fallback to single-step.
///
/// # Arguments
/// * `diff` - The git diff to generate a commit message for
/// * `max_tokens` - Maximum number of tokens allowed for the response
/// * `model` - The AI model to use for generation
/// * `settings` - Optional application settings to customize the request
///
/// # Returns
/// * `Result<openai::Response>` - The generated commit message or an error
///
/// # Errors
/// Returns an error if:
/// - max_tokens is 0
/// - OpenAI API call fails
pub async fn generate(patch: String, remaining_tokens: usize, _model: Model, settings: Option<&AppConfig>) -> Result<openai::Response> {
  profile!("Generate commit message");

  if remaining_tokens == 0 {
    bail!("Maximum token count must be greater than zero")
  }

  // Use the provided settings, or fall back to global config
  let config = settings.unwrap_or(&config::APP_CONFIG);

  // Use the new strategy pattern for generation
  let message = crate::generation::fallback::generate_with_fallback(&patch, config).await?;

  Ok(openai::Response { response: message })
}

pub fn token_used(model: &Model) -> Result<usize> {
  get_instruction_token_count(model)
}

/// Calculates the number of tokens used by the instruction template.
///
/// # Arguments
/// * `model` - The AI model to use for token counting
///
/// # Returns
/// * `Result<usize>` - The number of tokens used or an error
pub fn get_instruction_token_count(model: &Model) -> Result<usize> {
  profile!("Calculate instruction tokens");
  let template = get_instruction_template()?;
  model.count_tokens(&template)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_missing_api_key_fallback() {
    // Create settings with no API key
    let settings = AppConfig {
      openai_api_key:    None,
      model:             Some("gpt-4o-mini".to_string()),
      max_tokens:        Some(1024),
      max_commit_length: Some(72),
      timeout:           Some(30)
    };

    // Temporarily clear the environment variable
    let original_key = std::env::var("OPENAI_API_KEY").ok();
    std::env::remove_var("OPENAI_API_KEY");

    // Test that generate falls back to local generation when no API key is available
    let result = generate(
      "diff --git a/test.txt b/test.txt\n+Hello World".to_string(),
      1024,
      Model::GPT41Mini,
      Some(&settings)
    )
    .await;

    // Restore original environment variable if it existed
    if let Some(key) = original_key {
      std::env::set_var("OPENAI_API_KEY", key);
    }

    // Should succeed with local fallback
    assert!(
      result.is_ok(),
      "Expected fallback to local generation to succeed, got error: {:?}",
      result.err()
    );
    let response = result.unwrap();
    assert!(!response.response.is_empty(), "Expected non-empty commit message");
  }

  #[tokio::test]
  async fn test_invalid_api_key_fallback() {
    // Create settings with invalid API key
    let settings = AppConfig {
      openai_api_key:    Some("<PLACE HOLDER FOR YOUR API KEY>".to_string()),
      model:             Some("gpt-4o-mini".to_string()),
      max_tokens:        Some(1024),
      max_commit_length: Some(72),
      timeout:           Some(30)
    };

    // Test that generate falls back to local generation when API key is invalid
    let result = generate(
      "diff --git a/test.txt b/test.txt\n+Hello World".to_string(),
      1024,
      Model::GPT41Mini,
      Some(&settings)
    )
    .await;

    // Should succeed with local fallback
    assert!(
      result.is_ok(),
      "Expected fallback to local generation to succeed, got error: {:?}",
      result.err()
    );
    let response = result.unwrap();
    assert!(!response.response.is_empty(), "Expected non-empty commit message");
  }
}
