use std::sync::Mutex;

use anyhow::{anyhow, bail, Result};
use maplit::hashmap;
use mustache;
use once_cell::sync::Lazy;

use crate::{client, config, profile};
use crate::model::Model;

/// The instruction template included at compile time
const INSTRUCTION_TEMPLATE: &str = include_str!("../resources/prompt.md");

// Cache for compiled templates
static TEMPLATE_CACHE: Lazy<Mutex<Option<mustache::Template>>> = Lazy::new(|| Mutex::new(None));

/// Returns the instruction template for the AI model.
/// This template guides the model in generating appropriate commit messages.
pub fn get_instruction_template() -> Result<String> {
  profile!("Generate instruction template");

  let max_length = config::APP.max_commit_length.unwrap_or(72).to_string();

  // Get or compile template
  let template = {
    let mut cache = TEMPLATE_CACHE.lock().unwrap();
    if cache.is_none() {
      *cache = Some(mustache::compile_str(INSTRUCTION_TEMPLATE).map_err(|e| anyhow!("Template compilation error: {}", e))?);
    }
    cache.as_ref().unwrap().clone()
  };

  // Render template
  template
    .render_to_string(&hashmap! {
      "max_length" => max_length
    })
    .map_err(|e| anyhow!("Template rendering error: {}", e))
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
fn create_commit_request(diff: String, max_tokens: usize, model: Model) -> Result<client::Request> {
  profile!("Prepare request");
  let template = get_instruction_template()?;

  // Pre-allocate string with estimated capacity
  let mut full_prompt = String::with_capacity(template.len() + diff.len() + 100);
  full_prompt.push_str(&template);
  full_prompt.push_str("\n\nChanges to review:\n");
  full_prompt.push_str(&diff);

  Ok(client::Request {
    system: template,
    prompt: full_prompt,
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
pub async fn generate(patch: String, remaining_tokens: usize, model: Model) -> Result<client::Response> {
  profile!("Generate commit message");

  if remaining_tokens == 0 {
    bail!("Maximum token count must be greater than zero")
  }

  let request = create_commit_request(patch, remaining_tokens, model)?;
  client::call(request).await
}

pub fn token_used(model: &Model) -> Result<usize> {
  get_instruction_token_count(model)
}
