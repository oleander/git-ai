use anyhow::{bail, Result};

use crate::{config, openai};
use crate::model::{Model, Request, Response};

fn instruction() -> String {
  format!("You are an AI assistant that generates concise and meaningful git commit messages based on provided diffs. Please adhere to the following guidelines:

  - Structure: Begin with a clear, present-tense summary.
  - Content: Emphasize the changes and their rationale, excluding irrelevant details.
  - Consistency: Maintain uniformity in tense, punctuation, and capitalization.
  - Accuracy: Ensure the message accurately reflects the changes and their purpose.
  - Present tense, imperative mood. (e.g., 'Add x to y' instead of 'Added x to y')
  - Max {} chars in the output

  ## Output:

  Your output should be a commit message generated from the input diff and nothing else.

  ## Input:

  INPUT:", config::APP.max_commit_length.unwrap_or(72))
}

pub fn token_used(model: &Model) -> Result<usize> {
  model.count_tokens(&instruction())
}

pub async fn generate(diff: String, max_tokens: usize, model: Model) -> Result<Response> {
  if max_tokens == 0 {
    bail!("Max can't be zero (2)")
  }

  let request = Request {
    system: instruction(),
    prompt: diff,
    max_tokens,
    model
  };

  openai::call(request).await
}
