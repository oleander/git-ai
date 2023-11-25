use serde_json::{from_str, json, Value};
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;
use thiserror::Error;
use reqwest::Client;
use dotenv_codegen::dotenv;
use log::trace;
use std::io;

const API_URL: &str = "https://api.openai.com/v1/chat/completions";
const MODEL: &str = "gpt-4-1106-preview";

lazy_static! {
  static ref API_KEY: String = dotenv!("OPENAI_API_KEY").to_string();
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

fn build_commit_message_prompt(language: &str, max_length: usize) -> String {
  trace!("[prompt] Language: {}, Max length: {}", language, max_length);

  format!(
        "Generate a concise git commit message written in present tense for the following code diff with the given specifications below:\nMessage language: {}\nCommit message must be a maximum of {} characters.\nExclude anything unnecessary such as translation. Your entire response will be passed directly into git commit.",
        language, max_length
    )
}

fn json_payload(messages: Vec<ChatMessage>) -> Value {
  trace!("[json_payload] Messages: {:?}", messages);

  json!({ "model": MODEL, "messages": messages })
}

fn http_client() -> Result<Client, ChatError> {
  Client::builder().build().map_err(|_| ChatError::HttpClientBuildError)
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
  ResponseExtractionError
}

fn extract_message_from_response(response: &str) -> Result<String, ChatError> {
  trace!("[extract_message_from_response] Response: {}", response);

  let response: Value = from_str(response)?;
  response["choices"]
    .as_array()
    .and_then(|choices| choices.get(0))
    .and_then(|choice| choice["message"]["content"].as_str())
    .map(|s| s.to_string())
    .ok_or(ChatError::ResponseExtractionError)
}

// Fetches a commit message from the OpenAI API
async fn fetch_completion(payload: Value) -> Result<String, ChatError> {
  trace!("[fetch_completion] Payload: {:?}", payload);

  let response =
    http_client()?
      .post(API_URL)
      .bearer_auth(API_KEY.as_str())
      .json(&payload)
      .timeout(std::time::Duration::from_secs(10))
      .send()
      .await?
      .text()
      .await?;

  extract_message_from_response(&response)
}

// Generates a commit message from the OpenAI API
pub async fn suggested_commit_message(diff: String) -> Result<String, ChatError> {
  trace!("[suggested_commit_message] Generating commit message");

  let chat_prompt = build_commit_message_prompt("en", 72);
  let messages = vec![ChatMessage::new("system", chat_prompt), ChatMessage::new("user", diff)];
  let payload = json_payload(messages);
  let response = fetch_completion(payload).await?;

  Ok(response)
}
