use anyhow::{anyhow, bail, Result};
use maplit::hashmap;
use mustache;
use async_openai::Client;

use crate::{config, debug_output, openai, profile};
use crate::model::Model;
use crate::config::AppConfig;
use crate::multi_step_integration::{generate_commit_message_local, generate_commit_message_multi_step};

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
pub async fn generate(patch: String, remaining_tokens: usize, model: Model, settings: Option<&AppConfig>) -> Result<openai::Response> {
  profile!("Generate commit message");

  if remaining_tokens == 0 {
    bail!("Maximum token count must be greater than zero")
  }

  // Try multi-step approach first
  let max_length = settings
    .and_then(|s| s.max_commit_length)
    .or(config::APP_CONFIG.max_commit_length);

  // Use custom settings if provided
  if let Some(custom_settings) = settings {
    // Always try to create config - this will handle API key validation and fallback
    match openai::create_openai_config(custom_settings) {
      Ok(config) => {
        let client = Client::with_config(config);
        let model_str = model.to_string();

        match generate_commit_message_multi_step(&client, &model_str, &patch, max_length).await {
          Ok(message) => return Ok(openai::Response { response: message }),
          Err(e) => {
            // Check if it's an API key error
            if e.to_string().contains("invalid_api_key") || e.to_string().contains("Incorrect API key") {
              bail!("Invalid OpenAI API key. Set via:\n1. git-ai config set openai-api-key <key>\n2. OPENAI_API_KEY environment variable");
            }
            log::warn!("Multi-step generation with custom settings failed: {e}");
            if let Some(session) = debug_output::debug_session() {
              session.set_multi_step_error(e.to_string());
            }
          }
        }
      }
      Err(e) => {
        // If config creation fails due to API key, propagate the error
        return Err(e);
      }
    }
  } else {
    // Try with default settings
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
      if !api_key.is_empty() {
        let client = Client::new();
        let model_str = model.to_string();

        match generate_commit_message_multi_step(&client, &model_str, &patch, max_length).await {
          Ok(message) => return Ok(openai::Response { response: message }),
          Err(e) => {
            // Check if it's an API key error
            if e.to_string().contains("invalid_api_key") || e.to_string().contains("Incorrect API key") {
              bail!("Invalid OpenAI API key. Set via:\n1. git-ai config set openai-api-key <key>\n2. OPENAI_API_KEY environment variable");
            }
            log::warn!("Multi-step generation failed: {e}");
            if let Some(session) = debug_output::debug_session() {
              session.set_multi_step_error(e.to_string());
            }
          }
        }
      }
    }
  }

  // Try local multi-step generation
  match generate_commit_message_local(&patch, max_length) {
    Ok(message) => return Ok(openai::Response { response: message }),
    Err(e) => {
      log::warn!("Local multi-step generation failed: {e}");
    }
  }

  // Mark that we're using single-step fallback
  if let Some(session) = debug_output::debug_session() {
    session.set_single_step_success(true);
  }

  // Fallback to original single-step approach
  let request = create_commit_request(patch, remaining_tokens, model)?;

  // Use custom settings if provided, otherwise use global config
  match settings {
    Some(custom_settings) => {
      // Create a client with custom settings
      match openai::create_openai_config(custom_settings) {
        Ok(config) => openai::call_with_config(request, config).await,
        Err(e) => Err(e)
      }
    }
    None => {
      // Use the default global config
      openai::call(request).await
    }
  }
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
  async fn test_missing_api_key_error() {
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

    // Test that generate returns an error for missing API key
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

    assert!(result.is_err());
    let error_message = result.unwrap_err().to_string();
    assert!(
      error_message.contains("OpenAI API key not found") || error_message.contains("git-ai config set openai-api-key"),
      "Expected error message about missing API key, got: {}",
      error_message
    );
  }

  #[tokio::test]
  async fn test_invalid_api_key_error() {
    // Create settings with invalid API key
    let settings = AppConfig {
      openai_api_key:    Some("<PLACE HOLDER FOR YOUR API KEY>".to_string()),
      model:             Some("gpt-4o-mini".to_string()),
      max_tokens:        Some(1024),
      max_commit_length: Some(72),
      timeout:           Some(30)
    };

    // Test that generate returns an error for invalid API key
    let result = generate(
      "diff --git a/test.txt b/test.txt\n+Hello World".to_string(),
      1024,
      Model::GPT41Mini,
      Some(&settings)
    )
    .await;

    assert!(result.is_err());
    let error_message = result.unwrap_err().to_string();
    assert!(
      error_message.contains("OpenAI API key not found") || error_message.contains("git-ai config set openai-api-key"),
      "Expected error message about invalid API key, got: {}",
      error_message
    );
  }

  #[tokio::test]
  async fn test_api_key_fallback_to_env() {
    // Set a valid-looking API key in environment (won't actually work but should pass validation)
    std::env::set_var("OPENAI_API_KEY", "sk-test123456789012345678901234567890123456789012345678");
    
    // Create settings with placeholder API key - should fall back to env var
    let settings = AppConfig {
      openai_api_key:    Some("<PLACE HOLDER FOR YOUR API KEY>".to_string()),
      model:             Some("gpt-4o-mini".to_string()),
      max_tokens:        Some(1024),
      max_commit_length: Some(72),
      timeout:           Some(30)
    };

    // This should not fail immediately due to API key - it should use the env var
    // (It will likely fail later due to invalid API key, but not in the validation stage)
    let result = generate(
      "diff --git a/test.txt b/test.txt\n+Hello World".to_string(),
      1024,
      Model::GPT41Mini,
      Some(&settings)
    )
    .await;

    // Clean up
    std::env::remove_var("OPENAI_API_KEY");

    // The error should NOT be about missing API key configuration
    if let Err(e) = result {
      let error_msg = e.to_string();
      assert!(
        !error_msg.contains("OpenAI API key not found"),
        "Should not get 'key not found' error when env var is set, got: {}",
        error_msg
      );
    }
  }
}
