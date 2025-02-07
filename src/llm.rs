use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use ollama_rs::generation::completion::request::GenerationRequest;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProviderType {
  OpenAI,
  Ollama
}

#[derive(Debug, Clone)]
pub struct CompletionRequest {
  pub prompt:     String,
  pub system:     String,
  pub max_tokens: Option<u16>,
  pub model:      ModelType
}

#[derive(Debug, Clone)]
pub struct CompletionResponse {
  pub content: String,
  pub model:   ModelType
}

#[derive(Debug, Clone)]
pub enum ModelType {
  OpenAI(String),
  Ollama(String)
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
  pub name:     String,
  pub provider: ProviderType
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
  async fn generate_completion(&self, request: CompletionRequest) -> Result<CompletionResponse>;
  async fn supported_models(&self) -> Result<Vec<ModelInfo>>;
}

pub struct OllamaProvider {
  client: ollama_rs::Ollama
}

impl OllamaProvider {
  pub fn new() -> Self {
    Self { client: ollama_rs::Ollama::default() }
  }

  pub fn with_endpoint(host: String, port: u16) -> Self {
    Self { client: ollama_rs::Ollama::new(host, port) }
  }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
  async fn generate_completion(&self, request: CompletionRequest) -> Result<CompletionResponse> {
    let model_name = match request.model {
      ModelType::Ollama(name) => name,
      _ => return Err(anyhow::anyhow!("Invalid model type for Ollama provider"))
    };

    let generation_request = GenerationRequest::new(model_name.clone(), request.prompt);

    let response = self.client.generate(generation_request).await?;

    Ok(CompletionResponse {
      content: response.response,
      model:   ModelType::Ollama(model_name)
    })
  }

  async fn supported_models(&self) -> Result<Vec<ModelInfo>> {
    let models = self.client.list_local_models().await?;
    Ok(
      models
        .into_iter()
        .map(|m| ModelInfo { name: m.name, provider: ProviderType::Ollama })
        .collect()
    )
  }
}

#[cfg(test)]
mod tests {
  use tokio;

  use super::*;

  #[tokio::test]
  async fn test_ollama_provider_models() {
    let provider = OllamaProvider::new();
    match provider.supported_models().await {
      Ok(models) => {
        println!("Found {} Ollama models", models.len());
      }
      Err(e) => {
        println!("Skipping Ollama model test - no server available: {}", e);
      }
    }
  }

  #[tokio::test]
  async fn test_ollama_completion() {
    let provider = OllamaProvider::new();
    let request = CompletionRequest {
      prompt:     "Say hello".to_string(),
      system:     "You are a helpful assistant".to_string(),
      max_tokens: Some(100),
      model:      ModelType::Ollama("llama2:latest".to_string())
    };

    match provider.generate_completion(request).await {
      Ok(response) => {
        assert!(!response.content.is_empty(), "Response should not be empty");
        println!("Ollama response: {}", response.content);
      }
      Err(e) => {
        println!("Skipping Ollama completion test - no server available: {}", e);
      }
    }
  }
}
