use std::collections::HashMap;

use anyhow::{bail, Result};
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;
use async_trait::async_trait;

use crate::model::Model;

pub struct OllamaClient {
  client: Ollama
}

#[async_trait]
pub trait OllamaClientTrait {
  async fn generate(&self, model: Model, prompt: &str) -> Result<String>;
  async fn is_available(&self, model: Model) -> bool;
}

impl OllamaClient {
  pub fn new() -> Result<Self> {
    // Default to localhost:11434 which is Ollama's default
    let client = Ollama::default();
    Ok(Self { client })
  }

  pub async fn generate(&self, model: Model, prompt: &str) -> Result<String> {
    let model_name = <&str>::from(&model);
    let request = GenerationRequest::new(model_name.to_string(), prompt.to_string());
    let response = self.client.generate(request).await?;
    Ok(response.response)
  }

  pub async fn is_available(&self, model: Model) -> bool {
    // For now, just try to generate a simple test prompt
    // This is a workaround since the API doesn't have a direct way to check model availability
    let test_prompt = "test";
    self.generate(model, test_prompt).await.is_ok()
  }
}

#[async_trait]
impl OllamaClientTrait for OllamaClient {
  async fn generate(&self, model: Model, prompt: &str) -> Result<String> {
    let model_name = <&str>::from(&model);
    let request = GenerationRequest::new(model_name.to_string(), prompt.to_string());
    let response = self.client.generate(request).await?;
    Ok(response.response)
  }

  async fn is_available(&self, model: Model) -> bool {
    let test_prompt = "test";
    self.generate(model, test_prompt).await.is_ok()
  }
}
