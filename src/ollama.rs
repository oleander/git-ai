use anyhow::Result;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::options::GenerationOptions;
use ollama_rs::Ollama;
use async_trait::async_trait;

use crate::model::Model;
use crate::{Request, Response};

pub struct OllamaClient {
  client: Ollama
}

#[async_trait]
pub trait OllamaClientTrait {
  async fn generate(&self, model: Model, prompt: &str) -> Result<String>;
  async fn is_available(&self, model: Model) -> bool;
}

impl OllamaClient {
  pub async fn new() -> Result<Self> {
    let client = Ollama::default();
    Ok(Self { client })
  }

  pub async fn call(&self, request: Request) -> Result<Response> {
    let model = request.model.to_string();
    let prompt = format!("{}: {}", request.system, request.prompt);
    let options = GenerationOptions::default().tfs_z(0.0);
    let generation_request = GenerationRequest::new(model, prompt).options(options);
    let res = self.client.generate(generation_request).await?;
    Ok(Response { response: res.response })
  }

  pub async fn generate(&self, model: Model, prompt: &str) -> Result<String> {
    let model_name = <&str>::from(&model);
    let request = GenerationRequest::new(model_name.to_string(), prompt.to_string());
    let response = self.client.generate(request).await?;
    Ok(response.response)
  }

  pub async fn is_available(&self, model: Model) -> bool {
    let test_prompt = "test";
    self.generate(model, test_prompt).await.is_ok()
  }
}

#[async_trait]
impl OllamaClientTrait for OllamaClient {
  async fn generate(&self, model: Model, prompt: &str) -> Result<String> {
    self.generate(model, prompt).await
  }

  async fn is_available(&self, model: Model) -> bool {
    self.is_available(model).await
  }
}
