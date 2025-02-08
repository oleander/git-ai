use anyhow::{Context, Result};

use crate::model::Model;
use crate::ollama::OllamaClient;
use crate::openai::{self, Request as OpenAIRequest};

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
    Model::Llama2 | Model::CodeLlama | Model::Mistral | Model::DeepSeekR1_7B => {
      let client = OllamaClient::new()?;

      // For Ollama, we combine system and user prompts with clear roles
      let full_prompt = format!("### System:\n{}\n\n### User:\n{}\n\n### Assistant:", request.system, request.prompt);

      let response = client.generate(request.model, &full_prompt).await?;
      Ok(Response { response })
    }
    _ => {
      // For OpenAI models, use the existing OpenAI client
      let openai_request = OpenAIRequest {
        prompt:     request.prompt,
        system:     request.system,
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
    Model::Llama2 | Model::CodeLlama | Model::Mistral | Model::DeepSeekR1_7B => {
      if let Ok(client) = OllamaClient::new() {
        return client.is_available(model).await;
      }
      false
    }
    _ => true // OpenAI models are always considered available if API key is set
  }
}
