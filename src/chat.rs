use std::time::Duration;
use std::io;

use serde_json::{from_str, json, Value};
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;
use dotenv_codegen::dotenv;
use thiserror::Error;
use anyhow::Context;
use reqwest::Client;

use crate::config;

const API_URL: &str = "https://api.openai.com/v1/chat/completions";

lazy_static! {
  static ref MAX_LENGTH: u8 = dotenv!("MAX_LENGTH").parse::<u8>().unwrap();
  static ref TIMEOUT: u64 = dotenv!("TIMEOUT").parse::<u64>().unwrap();
  static ref API_KEY: String = dotenv!("OPENAI_API_KEY").to_string();
  static ref LANGUAGE: String = dotenv!("LANGUAGE").to_string();
  static ref MODEL: String = dotenv!("MODEL").to_string();
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

fn payload(diff: String) -> Value {
  let model = config::get("model").unwrap_or(MODEL.to_owned());

  json!({
     "model": model,
     "messages": vec![
       ChatMessage::new("system", prompt()),
       ChatMessage::new("user", diff)
     ]
  })
}

fn prompt() -> String {
  let lang = config::get("language").unwrap_or(LANGUAGE.as_str().to_owned());
  let length = config::get("max-length").unwrap_or(*MAX_LENGTH as i32);

  format!(
    "Generate a concise git commit message written in present tense for the following code diff with the given specifications below:\nMessage language: {:?}\nCommit message must be a maximum of {:?} characters.\nExclude anything unnecessary such as translation. Your entire response will be passed directly into git commit.",
    lang, length
  )
}

async fn response(diff: String) -> Result<Value, ChatError> {
  let api_key = config::get("api_key").unwrap_or(API_KEY.as_str().to_owned());
  let timeout = config::get("timeout").unwrap_or((*TIMEOUT) as i32);

  Client::builder()
    .build()?
    .post(API_URL)
    .bearer_auth(api_key)
    .json(&payload(diff))
    .timeout(Duration::from_secs(timeout as u64))
    .send()
    .await
    .context("Failed to send request")?
    .text()
    .await
    .context("Failed to get response body")
    .and_then(|body| from_str::<Value>(&body).context("Failed to parse JSON"))
    .map_err(|e| e.into())
}

pub async fn generate_commit(diff: String) -> Result<String, ChatError> {
  response(diff).await?["choices"]
    .as_array()
    .and_then(|choices| choices.first())
    .and_then(|choice| choice["message"]["content"].as_str())
    .map(|s| s.to_string())
    .ok_or(ChatError::ResponseExtractionError)
}
