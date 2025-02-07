use anyhow::{bail, Result};

use crate::{config, openai, profile};
use crate::model::Model;

const INSTRUCTION_TEMPLATE: &str = r#"You are an AI assistant that generates concise and meaningful git commit messages based on provided diffs. Please adhere to the following guidelines:

- Structure: Begin with a clear, present-tense summary.
- Content: Emphasize the changes and their rationale, excluding irrelevant details.
- Consistency: Maintain uniformity in tense, punctuation, and capitalization.
- Accuracy: Ensure the message accurately reflects the changes and their purpose.
- Present tense, imperative mood. (e.g., 'Add x to y' instead of 'Added x to y')
- Max {} chars in the output

## Output:

Your output should be a commit message generated from the input diff and nothing else.

## Input:

INPUT:"#;

/// Returns the instruction template for the AI model.
/// This template guides the model in generating appropriate commit messages.
fn get_instruction_template() -> String {
  profile!("Generate instruction template");
  INSTRUCTION_TEMPLATE.replace("{}", &config::APP.max_commit_length.unwrap_or(72).to_string())
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
  model.count_tokens(&get_instruction_template())
}

/// Creates an OpenAI request for commit message generation.
///
/// # Arguments
/// * `diff` - The git diff to generate a commit message for
/// * `max_tokens` - Maximum number of tokens allowed for the response
/// * `model` - The AI model to use for generation
///
/// # Returns
/// * `openai::Request` - The prepared request
fn create_commit_request(diff: String, max_tokens: usize, model: Model) -> openai::Request {
  profile!("Prepare OpenAI request");
  openai::Request {
    system: get_instruction_template(),
    prompt: diff,
    max_tokens: max_tokens.try_into().unwrap_or(u16::MAX),
    model
  }
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
pub async fn generate_commit_message(diff: String, max_tokens: usize, model: Model) -> Result<openai::Response> {
  profile!("Generate commit message");

  if max_tokens == 0 {
    bail!("Maximum token count must be greater than zero")
  }

  let request = create_commit_request(diff, max_tokens, model);
  openai::call(request).await
}
