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
  #[error("Failed to extract message from response body")]
  ResponseExtractionError,
  #[error("Anyhow error: {0}")]
  Anyhow(#[from] anyhow::Error),
  #[error("OpenAI error: {0}")]
  OpenAIError(String)
}

fn payload(diff: String) -> Value {
  let model = config::APP.model.clone();

  json!({
     "model": model,
     "messages": vec![
       ChatMessage::new("system", prompt()),
       ChatMessage::new("user", diff)
     ]
  })
}

fn prompt() -> String {
  let lang = config::APP.language.clone();
  let length = config::APP.max_length;

  format!(
    "Generate a concise git commit message written in present tense for the following code diff with the given specifications below:\nMessage language: {:?}\nCommit message must be a maximum of {:?} characters.\nExclude anything unnecessary such as translation. Your entire response will be passed directly into git commit.",
    lang, length
  )
}

mod response {
  use super::*;

  #[derive(Debug, Serialize, Deserialize)]
  pub struct Success {
    system_fingerprint: String,
    pub choices:        Vec<Choice>,
    object:             String,
    model:              String,
    id:                 String,
    usage:              Usage
  }

  #[derive(Debug, Serialize, Deserialize)]
  pub struct Error {
    pub error:   String,
    code:        usize,
    pub message: String
  }

  #[derive(Debug, Serialize, Deserialize)]
  pub struct Usage {
    completion_tokens: usize,
    prompt_tokens:     usize,
    total_tokens:      usize
  }

  #[derive(Debug, Serialize, Deserialize)]
  pub struct Choice {
    finish_reason: String,
    index:         usize,
    pub message:   Message
  }

  #[derive(Debug, Serialize, Deserialize)]
  pub struct Message {
    pub content: String,
    role:        String
  }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
  Success(response::Success),
  Error(response::Error)
}

async fn response(diff: String) -> Result<Response, ChatError> {
  let api_key = config::APP.openai_api_key.clone();
  let timeout = config::APP.timeout;

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
    .and_then(|body| from_str::<Response>(&body).context(format!("Failed to parse response body: {}", body)))
    .map_err(ChatError::from)
}

pub async fn generate_commit(diff: String) -> Result<String, ChatError> {
  match response(diff).await? {
    Response::Success(success) => Ok(success.choices.first().map(|choice| choice.message.content.clone()).unwrap()),
    Response::Error(error) => Err(ChatError::OpenAIError(error.message))
  }
}
