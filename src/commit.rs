use anyhow::{anyhow, bail, Result};
use maplit::hashmap;
use mustache;

use crate::{config, openai, profile};
use crate::model::Model;

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
///
/// # Arguments
/// * `diff` - The git diff to generate a commit message for
/// * `max_tokens` - Maximum number of tokens allowed for the response
/// * `model` - The AI model to use for generation
///
/// # Returns
/// * `Result<openai::Response>` - The generated commit message or an error
///
/// # Errors
/// Returns an error if:
/// - max_tokens is 0
/// - OpenAI API call fails
pub async fn generate(patch: String, remaining_tokens: usize, model: Model) -> Result<openai::Response> {
  profile!("Generate commit message");

  if remaining_tokens == 0 {
    bail!("Maximum token count must be greater than zero")
  }

  let request = create_commit_request(patch, remaining_tokens, model)?;
  openai::call(request).await
}

pub fn token_used(model: &Model) -> Result<usize> {
  get_instruction_token_count(model)
}
