use ai::llm::{CompletionRequest, LLMProvider, ModelType, OllamaProvider, ProviderType};

#[tokio::test]
async fn test_ollama_provider_creation() {
  // Test default provider
  let provider = OllamaProvider::new();
  assert!(provider.supported_models().await.is_ok());

  // Test custom endpoint provider
  let provider = OllamaProvider::with_endpoint("http://localhost".to_string(), 11434);
  assert!(provider.supported_models().await.is_ok());
}

#[tokio::test]
async fn test_ollama_model_listing() {
  let provider = OllamaProvider::new();
  match provider.supported_models().await {
    Ok(models) => {
      println!("Found models: {:?}", models);
      for model in models {
        assert_eq!(model.provider, ProviderType::Ollama);
        assert!(!model.name.is_empty());
      }
    }
    Err(e) => {
      println!("Skipping test - no Ollama server available: {}", e);
    }
  }
}

#[tokio::test]
async fn test_ollama_completion_with_system_prompt() {
  let provider = OllamaProvider::new();
  let request = CompletionRequest {
    prompt:     "What is Rust?".to_string(),
    system:     "You are a programming language expert. Keep answers short and technical.".to_string(),
    max_tokens: Some(100),
    model:      ModelType::Ollama("llama2:latest".to_string())
  };

  match provider.generate_completion(request).await {
    Ok(response) => {
      assert!(!response.content.is_empty());
      assert!(matches!(response.model, ModelType::Ollama(_)));
      println!("Response: {}", response.content);
    }
    Err(e) => {
      println!("Skipping test - no Ollama server available: {}", e);
    }
  }
}

#[tokio::test]
async fn test_ollama_completion_without_max_tokens() {
  let provider = OllamaProvider::new();
  let request = CompletionRequest {
    prompt:     "Write a haiku about Rust".to_string(),
    system:     "You are a poet.".to_string(),
    max_tokens: None,
    model:      ModelType::Ollama("llama2:latest".to_string())
  };

  match provider.generate_completion(request).await {
    Ok(response) => {
      assert!(!response.content.is_empty());
      println!("Haiku: {}", response.content);
    }
    Err(e) => {
      println!("Skipping test - no Ollama server available: {}", e);
    }
  }
}

#[tokio::test]
async fn test_ollama_invalid_model() {
  let provider = OllamaProvider::new();
  let request = CompletionRequest {
    prompt:     "Hello".to_string(),
    system:     "Be brief".to_string(),
    max_tokens: None,
    model:      ModelType::OpenAI("gpt-4".to_string()) // Intentionally wrong model type
  };

  let result = provider.generate_completion(request).await;
  assert!(result.is_err());
  if let Err(e) = result {
    assert!(e.to_string().contains("Invalid model type"));
  }
}

#[tokio::test]
async fn test_ollama_git_commit_scenario() {
  let provider = OllamaProvider::new();
  let diff = r#"
diff --git a/src/llm.rs b/src/llm.rs
new file mode 100644
index 0000000..1234567
--- /dev/null
+++ b/src/llm.rs
@@ +1,50 @@
+use anyhow::Result;
+use async_trait::async_trait;
+
+pub trait LLMProvider {
+    async fn generate_completion(&self) -> Result<String>;
+}
"#;

  let request = CompletionRequest {
    prompt:     diff.to_string(),
    system:     "You are a git commit message generator. Be concise and clear.".to_string(),
    max_tokens: Some(100),
    model:      ModelType::Ollama("llama2:latest".to_string())
  };

  match provider.generate_completion(request).await {
    Ok(response) => {
      assert!(!response.content.is_empty());
      println!("Commit message: {}", response.content);
    }
    Err(e) => {
      println!("Skipping test - no Ollama server available: {}", e);
    }
  }
}

/// Helper function to check if Ollama server is available
async fn is_ollama_available() -> bool {
  let provider = OllamaProvider::new();
  provider.supported_models().await.is_ok()
}

#[tokio::test]
async fn test_ollama_error_handling() {
  // Only run this test if Ollama is not available
  if !is_ollama_available().await {
    let provider = OllamaProvider::new();
    let request = CompletionRequest {
      prompt:     "Hello".to_string(),
      system:     "Be brief".to_string(),
      max_tokens: None,
      model:      ModelType::Ollama("llama2:latest".to_string())
    };

    let result = provider.generate_completion(request).await;
    assert!(result.is_err());
    if let Err(e) = result {
      assert!(
        e.to_string().contains("Connection refused")
          || e.to_string().contains("Failed to connect")
          || e.to_string().contains("Connection error")
      );
    }
  }
}
