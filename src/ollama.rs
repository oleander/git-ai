<<<<<<< HEAD
use anyhow::Result;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::options::GenerationOptions;
use ollama_rs::Ollama;

use crate::model::{Request, Response};

pub async fn call(request: Request) -> Result<Response> {
  let ollama = Ollama::default();

  let model = request.model.to_string();
  let prompt = format!("{}: {}", request.system, request.prompt);

  let options = GenerationOptions::default();

  let generation_request = GenerationRequest::new(model, prompt).options(options);

  let res = ollama.generate(generation_request).await?;

  Ok(Response { response: res.response })
=======
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
>>>>>>> fbe8ab1 (<think>)
}
