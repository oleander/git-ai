use anyhow::{bail, Result};

use crate::{config, openai};
use crate::model::Model;
use crate::llm::{CompletionRequest, CompletionResponse, LLMProvider, ModelType};

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

pub async fn generate(diff: String, max_tokens: usize, model: Model) -> Result<CompletionResponse> {
  if max_tokens == 0 {
    bail!("Max can't be zero (2)")
  }

  let provider: Box<dyn LLMProvider> = match config::APP.provider.as_deref() {
    Some("openai") | None => {
      let api_key = config::APP
        .openai
        .api_key
        .clone()
        .ok_or_else(|| anyhow::anyhow!("OpenAI API key not found"))?;
      Box::new(openai::OpenAIProvider::new(api_key))
    }
    Some("ollama") => Box::new(crate::llm::OllamaProvider::new()),
    Some(other) => bail!("Unknown provider: {}", other)
  };

  let model_type = match config::APP.provider.as_deref() {
    Some("openai") | None => ModelType::OpenAI(model.to_string()),
    Some("ollama") =>
      ModelType::Ollama(
        config::APP
          .ollama
          .model
          .clone()
          .unwrap_or_else(|| "llama2".to_string())
      ),
    Some(_) => unreachable!()
  };

  let request = CompletionRequest {
    system:     instruction(),
    prompt:     diff,
    max_tokens: Some(max_tokens.try_into().unwrap_or(u16::MAX)),
    model:      model_type
  };

  provider.generate_completion(request).await
}
