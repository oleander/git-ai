use anyhow::Result;
use serde::Serialize;
use {reqwest, serde_json};
use thiserror::Error;

use crate::{commit, config, profile};
use crate::model::Model;

const MAX_ATTEMPTS: usize = 3;

#[derive(Error, Debug)]
pub enum OpenAIError {
    #[error("Failed to connect to OpenAI API at {url}. Please check:\n1. The URL is correct and accessible\n2. Your network connection is working\n3. The API endpoint supports chat completions\n\nError details: {source}")]
    ConnectionError {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("Invalid response from OpenAI API: {0}")]
    InvalidResponse(String),
    #[error("OpenAI API key not set. Please set it using:\ngit ai config set openai-api-key <YOUR_API_KEY>")]
    MissingApiKey,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Response {
  pub response: String
}

#[derive(Debug, Serialize)]
pub struct Request {
  pub model:       String,
  pub messages:    Vec<Message>,
  pub max_tokens:  u16,
  pub temperature: f32
}

impl Request {
  pub fn new(model: Model, system: String, prompt: String, max_tokens: u16) -> Self {
    Self {
      model: model.to_string(),
      messages: vec![Message { role: "system".to_string(), content: system }, Message { role: "user".to_string(), content: prompt }],
      max_tokens,
      temperature: 0.7
    }
  }
}

#[derive(Debug, Serialize)]
pub struct Message {
  pub role:    String,
  pub content: String
}

/// Generates an improved commit message using the provided prompt and diff
pub async fn generate_commit_message(diff: &str) -> Result<String> {
  profile!("Generate commit message");
  let response = commit::generate(diff.into(), 256, Model::GPT4oMini).await?;
  Ok(response.response.trim().to_string())
}

fn truncate_to_fit(text: &str, max_tokens: usize, model: &Model) -> Result<String> {
  let token_count = model.count_tokens(text)?;
  if token_count <= max_tokens {
    return Ok(text.to_string());
  }

  let lines: Vec<&str> = text.lines().collect();
  if lines.is_empty() {
    return Ok(String::new());
  }

  // Try increasingly aggressive truncation until we fit
  for attempt in 0..MAX_ATTEMPTS {
    let keep_lines = match attempt {
      0 => lines.len() * 3 / 4, // First try: Keep 75%
      1 => lines.len() / 2,     // Second try: Keep 50%
      _ => lines.len() / 4      // Final try: Keep 25%
    };

    if keep_lines == 0 {
      break;
    }

    let mut truncated = Vec::new();
    truncated.extend(lines.iter().take(keep_lines));
    truncated.push("... (truncated for length) ...");

    let result = truncated.join("\n");
    let new_token_count = model.count_tokens(&result)?;

    if new_token_count <= max_tokens {
      return Ok(result);
    }
  }

  // If standard truncation failed, do minimal version with iterative reduction
  let mut minimal = Vec::new();
  let mut current_size = lines.len() / 50; // Start with 2% of lines

  while current_size > 0 {
    minimal.clear();
    minimal.extend(lines.iter().take(current_size));
    minimal.push("... (severely truncated for length) ...");

    let result = minimal.join("\n");
    let new_token_count = model.count_tokens(&result)?;

    if new_token_count <= max_tokens {
      return Ok(result);
    }

    current_size /= 2; // Halve the size each time
  }

  // If everything fails, return just the truncation message
  Ok("... (content too large, completely truncated) ...".to_string())
}

pub async fn call(request: Request) -> Result<Response> {
  profile!("OpenAI API call");
  let client = reqwest::Client::new();
  let openai_key = config::APP
    .openai
    .api_key
    .clone()
    .ok_or(OpenAIError::MissingApiKey)?;

  let openai_host = config::APP.openai.host.clone();
  let url = format!("{}/chat/completions", openai_host);

  let response = client
    .post(&url)
    .header("Authorization", format!("Bearer {}", openai_key))
    .header("Content-Type", "application/json")
    .json(&request)
    .send()
    .await
    .map_err(|e| OpenAIError::ConnectionError {
      url: url.clone(),
      source: e,
    })?;

  let response = response.json::<serde_json::Value>().await
    .map_err(|e| OpenAIError::InvalidResponse(e.to_string()))?;

  let content = response["choices"][0]["message"]["content"]
    .as_str()
    .ok_or_else(|| OpenAIError::InvalidResponse("Response missing expected 'choices[0].message.content' field".to_string()))?
    .to_string();

  Ok(Response { response: content })
}
