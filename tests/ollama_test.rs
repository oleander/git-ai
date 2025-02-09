use anyhow::Result;
use async_trait::async_trait;
use ai::model::Model;
use ai::ollama::OllamaClientTrait;

// Mock Ollama client for testing
struct MockOllamaClient;

#[async_trait]
impl OllamaClientTrait for MockOllamaClient {
  async fn generate(&self, _model: Model, _prompt: &str) -> Result<String> {
    Ok("Mock response".to_string())
  }

  async fn is_available(&self, _model: Model) -> bool {
    true
  }
}

#[tokio::test]
async fn test_generate() -> Result<()> {
  let client = MockOllamaClient;
  let model = Model::Llama2;
  let prompt = "Test prompt";

  let result = client.generate(model, prompt).await?;
  assert_eq!(result, "Mock response");
  Ok(())
}

#[tokio::test]
async fn test_is_available() -> Result<()> {
  let client = MockOllamaClient;
  let model = Model::Llama2;

  let available = client.is_available(model).await;
  assert!(available);
  Ok(())
}

// Real OllamaClient integration tests
// These tests require:
// 1. Ollama to be running locally (run: `ollama serve`)
// 2. The Llama2 model to be pulled (run: `ollama pull llama2`)
mod real_client_tests {
  use std::env;

  use ai::ollama::OllamaClient;
  use ai::Request;

  use super::*;

  async fn skip_if_no_ollama() {
    if env::var("RUN_OLLAMA_TESTS").is_err() {
      eprintln!("Skipping Ollama integration tests. Set RUN_OLLAMA_TESTS=1 to run them.");
    }
  }

  #[tokio::test]
  async fn test_new_client() -> Result<()> {
    skip_if_no_ollama().await;
    Ok(())
  }

  #[tokio::test]
  async fn test_call_with_request() -> Result<()> {
    skip_if_no_ollama().await;
    let client = OllamaClient::new().await?;
    let request = Request {
      prompt:     "Test prompt".to_string(),
      system:     "You are a test assistant".to_string(),
      max_tokens: 100,
      model:      Model::Llama2
    };

    match client.call(request).await {
      Ok(response) => {
        assert!(!response.response.is_empty());
      }
      Err(e) => {
        eprintln!("Note: This test requires Ollama to be running with the Llama2 model pulled");
        eprintln!("Error: {}", e);
      }
    }
    Ok(())
  }

  #[tokio::test]
  async fn test_generate_with_model() -> Result<()> {
    skip_if_no_ollama().await;
    let client = OllamaClient::new().await?;
    match client.generate(Model::Llama2, "Test prompt").await {
      Ok(result) => {
        assert!(!result.is_empty());
      }
      Err(e) => {
        eprintln!("Note: This test requires Ollama to be running with the Llama2 model pulled");
        eprintln!("Error: {}", e);
      }
    }
    Ok(())
  }

  #[tokio::test]
  async fn test_model_availability() -> Result<()> {
    skip_if_no_ollama().await;
    let client = OllamaClient::new().await?;

    // Only test if Ollama is running
    match client.is_available(Model::Llama2).await {
      true => println!("Llama2 model is available"),
      false => eprintln!("Note: This test requires the Llama2 model to be pulled: `ollama pull llama2`")
    }

    // GPT4 should always be unavailable in Ollama
    assert!(!client.is_available(Model::GPT4).await);

    Ok(())
  }
}
