use async_openai::types::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use async_openai::config::OpenAIConfig;
use async_openai::Client;
use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::llm::{CompletionRequest, CompletionResponse, LLMProvider, ModelInfo, ModelType, ProviderType};

#[derive(Debug, Clone)]
pub struct OpenAIProvider {
  client: Client<OpenAIConfig>
}

impl OpenAIProvider {
  pub fn new(api_key: String) -> Self {
    let config = OpenAIConfig::new().with_api_key(api_key);
    Self { client: Client::with_config(config) }
  }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
  async fn generate_completion(&self, request: CompletionRequest) -> Result<CompletionResponse> {
    let model_name = match request.model {
      ModelType::OpenAI(name) => name,
      _ => return Err(anyhow::anyhow!("Invalid model type for OpenAI provider"))
    };

    let request = CreateChatCompletionRequestArgs::default()
      .model(model_name.clone())
      .max_tokens(request.max_tokens.unwrap_or(1000))
      .messages([
        ChatCompletionRequestSystemMessageArgs::default()
          .content(request.system)
          .build()?
          .into(),
        ChatCompletionRequestUserMessageArgs::default()
          .content(request.prompt)
          .build()?
          .into()
      ])
      .build()?;

    let chat = self.client.chat().create(request).await?;
    let choice = chat
      .choices
      .first()
      .context("No completion choices returned")?;

    Ok(CompletionResponse {
      content: choice
        .message
        .content
        .clone()
        .context("No content in completion response")?,
      model:   ModelType::OpenAI(model_name)
    })
  }

  async fn supported_models(&self) -> Result<Vec<ModelInfo>> {
    // For now, return a static list of supported models
    Ok(vec![
      ModelInfo {
        name:     "gpt-4".to_string(),
        provider: ProviderType::OpenAI
      },
      ModelInfo {
        name:     "gpt-4-turbo-preview".to_string(),
        provider: ProviderType::OpenAI
      },
    ])
  }
}

#[cfg(test)]
mod tests {
  use std::env;

  use super::*;

  #[tokio::test]
  async fn test_openai_completion() {
    match env::var("OPENAI_API_KEY") {
      Ok(api_key) => {
        let provider = OpenAIProvider::new(api_key);
        let request = CompletionRequest {
          prompt:     "Say hello".to_string(),
          system:     "You are a helpful assistant".to_string(),
          max_tokens: Some(100),
          model:      ModelType::OpenAI("gpt-4".to_string())
        };

        match provider.generate_completion(request).await {
          Ok(response) => {
            assert!(!response.content.is_empty(), "Response should not be empty");
            println!("OpenAI response: {}", response.content);
          }
          Err(e) => {
            println!("Skipping OpenAI test - API key may be invalid: {}", e);
          }
        }
      }
      Err(_) => {
        println!("Skipping OpenAI test - no API key available");
      }
    }
  }
}
