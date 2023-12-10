use std::io;

use anyhow::Context;
use serde_json::{from_str, json, Value};
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;
use dotenv_codegen::dotenv;
use thiserror::Error;
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
  OpenAIError(String),
  #[error("Failed to parse response: {1} ({0})")]
  ParseError(serde_json::Error, String)
}

fn payload(diff: String) -> Value {
  let model = config::APP.model.clone();

  json!({
    "model": model,
    "messages": vec![
      json!({
        "role": "system",
        "content": prompt()
      }),
      json!({
        "role": "user",
        "content": diff
      })
    ]
  })
}

fn prompt() -> String {
  let lang = config::APP.language.clone();
  let length = config::APP.max_length;

  format!(
    "
    Create a concise git commit message in present tense for the provided code diff. 
      Follow these guidelines:
        Language: {}.
        Maximum Length: {} characters.
        Clearly detail what changes were made and why.
        Exclude irrelevant and unnecessary details, such as translations.
      Your entire response will be passed directly into git commit.",
    lang, length
  )
  .split_whitespace()
  .collect::<Vec<&str>>()
  .join(" ")
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
  let api_key = config::APP.openai_api_key.clone().context("Failed to get OpenAI API key")?;
  let timeout = config::APP.duration();

  Client::builder()
    .build()?
    .post(API_URL)
    .bearer_auth(api_key)
    .json(&payload(diff))
    .timeout(timeout)
    .send()
    .await
    .map_err(ChatError::from)?
    .text()
    .await
    .map_err(ChatError::from)
    .and_then(|body| from_str::<Response>(&body).map_err(|e| ChatError::ParseError(e, body)))
}

pub async fn generate_commit(diff: String) -> Result<String, ChatError> {
  match response(diff).await? {
    Response::Success(success) => Ok(success.choices.first().map(|choice| choice.message.content.clone()).unwrap()),
    Response::Error(error) => Err(ChatError::OpenAIError(error.message))
  }
}
