use anyhow::{anyhow, bail, Result};
use maplit::hashmap;
use mustache;
use async_openai::Client;

use crate::{config, debug_output, openai, profile};
use crate::model::Model;
use crate::config::App as Settings;
use crate::multi_step_integration::{generate_commit_message_local, generate_commit_message_multi_step};

/// The instruction template included at compile time
const INSTRUCTION_TEMPLATE: &str = include_str!("../resources/prompt.md");

/// Returns the instruction template for the AI model.
/// This template guides the model in generating appropriate commit messages.
fn get_instruction_template() -> Result<String> {
  profile!("Generate instruction template");
  let max_length = config::APP.max_commit_length.unwrap_or(72).to_string();
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
fn create_commit_request(diff: String, max_tokens: usize, model: Model) -> Result<openai::Request> {
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
pub async fn generate(patch: String, remaining_tokens: usize, model: Model, settings: Option<&Settings>) -> Result<openai::Response> {
  profile!("Generate commit message");

  if remaining_tokens == 0 {
    bail!("Maximum token count must be greater than zero")
  }

  // Try multi-step approach first
  let max_length = settings
    .and_then(|s| s.max_commit_length)
    .or(config::APP.max_commit_length);

  // Use custom settings if provided
  if let Some(custom_settings) = settings {
    if let Some(api_key) = &custom_settings.openai_api_key {
      if !api_key.is_empty() && api_key != "<PLACE HOLDER FOR YOUR API KEY>" {
        let config = openai::create_openai_config(custom_settings)?;
        let client = Client::with_config(config);
        let model_str = model.to_string();

        match generate_commit_message_multi_step(&client, &model_str, &patch, max_length).await {
          Ok(message) => return Ok(openai::Response { response: message }),
          Err(e) => {
            log::warn!("Multi-step generation with custom settings failed: {e}");
            if let Some(session) = debug_output::debug_session() {
              session.set_multi_step_error(e.to_string());
            }
          }
        }
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
      let config = openai::create_openai_config(custom_settings)?;
      openai::call_with_config(request, config).await
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
