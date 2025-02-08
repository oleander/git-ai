use anyhow::{bail, Context, Result};

use crate::model::Model;
use crate::ollama::OllamaClient;
use crate::openai::{self, Request as OpenAIRequest};
use crate::commit;

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
  pub prompt:     String,
  pub system:     String,
  pub max_tokens: u16,
  pub model:      Model
}

#[derive(Debug, Clone, PartialEq)]
pub struct Response {
  pub response: String
}

pub async fn call(request: Request) -> Result<Response> {
  match request.model {
    Model::Llama2 | Model::CodeLlama | Model::Mistral | Model::DeepSeekR1_7B | Model::SmollM2 | Model::Tavernari => {
      let client = OllamaClient::new()?;

      let template = commit::get_instruction_template()?;
      let full_prompt = format!(
        "{}\n\nImportant: Respond with ONLY a single line containing the commit message. Do not include any other text, formatting, or explanation.\n\nChanges to review:\n{}",
        template,
        request.prompt
      );
      let response = client.generate(request.model, &full_prompt).await?;

      // Log the raw response for debugging
      log::debug!("Raw Ollama response: {}", response);

      // Take the first non-empty line as the commit message, trimming any whitespace
      let commit_message = response.trim().to_string();

      if commit_message.is_empty() {
        bail!("Model returned an empty response");
      }

      Ok(Response { response: commit_message })
    }
    _ => {
      // For OpenAI models, use the instruction template as the system prompt
      let template = commit::get_instruction_template()?;
      let openai_request = OpenAIRequest {
        prompt:     request.prompt,
        system:     template,
        max_tokens: request.max_tokens,
        model:      request.model
      };

      let response = openai::call(openai_request).await?;
      Ok(Response { response: response.response })
    }
  }
}

pub async fn is_model_available(model: Model) -> bool {
  match model {
    Model::Llama2 | Model::CodeLlama | Model::Mistral | Model::DeepSeekR1_7B | Model::SmollM2 => {
      if let Ok(client) = OllamaClient::new() {
        return client.is_available(model).await;
      }
      false
    }
    _ => true // OpenAI models are always considered available if API key is set
  }
}
