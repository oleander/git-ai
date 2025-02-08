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
    Model::Llama2 | Model::CodeLlama | Model::Mistral | Model::DeepSeekR1_7B => {
      let client = OllamaClient::new()?;

      // For Ollama, we combine system and user prompts with clear roles and request JSON output
      let full_prompt = format!(
        "### System:\n{}\n\nIMPORTANT: You are a JSON-only assistant. Your response must be a valid JSON object with exactly one field named 'commit_message'. Example:\n{{\n  \"commit_message\": \"feat: add new feature\"\n}}\n\nRules:\n1. Start your response with '{{'\n2. End your response with '}}'\n3. Include ONLY the JSON object\n4. No other text or explanation\n5. No markdown formatting\n\n### User:\n{}\n\n### Assistant:\n",
        request.system,
        request.prompt
      );

      let response = client.generate(request.model, &full_prompt).await?;

      // Log the raw response for debugging
      log::debug!("Raw Ollama response: {}", response);

      // Try to extract JSON from the response by finding the first '{' and last '}'
      let json_str = response
        .find('{')
        .and_then(|start| response.rfind('}').map(|end| &response[start..=end]))
        .with_context(|| format!("Could not find JSON object in response: {}", response))?;

      log::debug!("Extracted JSON string: {}", json_str);

      // Parse the JSON response
      let json_response: serde_json::Value =
        serde_json::from_str(json_str).with_context(|| format!("Failed to parse JSON response from Ollama: {}", json_str))?;

      // Extract the commit message from the JSON
      let commit_message = json_response["commit_message"]
        .as_str()
        .with_context(|| format!("Failed to extract commit_message from JSON response: {}", json_str))?;

      Ok(Response { response: commit_message.to_string() })
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
