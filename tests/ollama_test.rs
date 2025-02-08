use anyhow::Result;
use async_trait::async_trait;
use ai::model::Model;
use ai::ollama::OllamaClientTrait;

// Mock Ollama client
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
  let model = Model::Llama2; // Use a valid model for testing
  let prompt = "Test prompt";

  let result = client.generate(model, prompt).await;
  assert!(result.is_ok());
  Ok(())
}

#[tokio::test]
async fn test_is_available() -> Result<()> {
  let client = MockOllamaClient;
  let model = Model::Llama2; // Use a valid model for testing

  let available = client.is_available(model).await;
  assert!(available);
  Ok(())
}
