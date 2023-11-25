use serde_json::{from_str, json, Value};
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;
use thiserror::Error;
use reqwest::Client;
use anyhow::Context;
use dotenv_codegen::dotenv;
use std::time::Duration;
use std::io;

const API_URL: &str = "https://api.openai.com/v1/chat/completions";
const MODEL: &str = "gpt-4-1106-preview";

lazy_static! {
  static ref TIMEOUT: u64 = dotenv!("TIMEOUT").parse::<u64>().unwrap();
   #[derive(Debug)]
  static ref API_KEY: String = dotenv!("OPENAI_API_KEY").to_string();
   #[derive(Debug)]
  static ref LANGUAGE: String = dotenv!("LANGUAGE").to_string();
   #[derive(Debug)]
  static ref MAX_LENGTH: u8 = dotenv!("MAX_LENGTH").parse::<u8>().unwrap();
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChatMessage {
  role:    String,
  content: String
}

impl ChatMessage {
  fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
    Self {
      role: role.into(), content: content.into()
    }
  }
}

#[derive(Error, Debug)]
pub enum ChatError {
  #[error("Failed to build HTTP client")]
  HttpClientBuildError,
  #[error("HTTP error: {0}")]
  HttpRequestError(#[from] reqwest::Error),
  #[error("IO error: {0}")]
  IOError(#[from] io::Error),
  #[error("Failed to parse JSON: {0}")]
  JsonParseError(#[from] serde_json::Error),
  #[error("Failed to extract message from response")]
  ResponseExtractionError,

  #[error("Anyhow error: {0}")]
  Anyhow(#[from] anyhow::Error)
}

// Generates a commit message from the OpenAI API
pub async fn suggested_commit_message(diff: String) -> Result<String, ChatError> {
  let prompt = format!(
    "Generate a concise git commit message written in present tense for the following code diff with the given specifications below:\nMessage language: {:?}\nCommit message must be a maximum of {:?} characters.\nExclude anything unnecessary such as translation. Your entire response will be passed directly into git commit.",
    LANGUAGE, MAX_LENGTH
  );

  let payload = json!({
     "model": MODEL,
     "messages": vec![
       ChatMessage::new("system", prompt),
       ChatMessage::new("user", diff)
     ]
  });

  let response = Client::builder()
    .build()?
    .post(API_URL)
    .bearer_auth(API_KEY.as_str())
    .json(&payload)
    .timeout(Duration::from_secs(*TIMEOUT))
    .send()
    .await
    .context("Failed to send request")?
    .text()
    .await
    .context("Failed to get response body")
    .and_then(|body| from_str::<Value>(&body).context("Failed to parse JSON"))?;

  let message = response["choices"]
    .as_array()
    .and_then(|choices| choices.get(0))
    .and_then(|choice| choice["message"]["content"].as_str())
    .map(|s| s.to_string())
    .ok_or(ChatError::ResponseExtractionError)?;

  Ok(message)
}
