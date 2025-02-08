use anyhow::{Context, Result};
use serde_json;

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
    Model::Llama2 | Model::CodeLlama | Model::Mistral | Model::DeepSeekR1_7B | Model::SmollM2 | Model::Tavernari => {
      let client = OllamaClient::new()?;

      // For Ollama, we combine system and user prompts with clear roles and request JSON output
      let full_prompt = format!(
        "### System:\n{}\n\nIMPORTANT: You are a commit message assistant. Your response must be EXACTLY ONE LINE containing ONLY the commit message. No other text, no JSON, no code blocks, no explanation. Just the commit message.\n\nExample good response:\nAdd user authentication feature\n\nExample bad responses:\n1. {{\"commit_message\": \"Add feature\"}}\n2. ```\nAdd feature\n```\n3. Here's the commit message: Add feature\n\nRemember: ONLY the commit message on a single line, nothing else. Keep it concise and clear.\n\n### User:\n{}\n\n### Assistant:\n",
        request.system,
        request.prompt
      );

      let response = client.generate(request.model, &full_prompt).await?;

      // Log the raw response for debugging
      log::debug!("Raw Ollama response: {}", response);

      // Take the first non-empty line as the commit message
      let commit_message = response
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .with_context(|| format!("Could not find commit message in response: {}", response))?;

      Ok(Response { response: commit_message })
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
    Model::Llama2 | Model::CodeLlama | Model::Mistral | Model::DeepSeekR1_7B | Model::SmollM2 => {
      if let Ok(client) = OllamaClient::new() {
        return client.is_available(model).await;
      }
      false
    }
    _ => true // OpenAI models are always considered available if API key is set
  }
}
