use http_cache_reqwest::{
  CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions
};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, Value};
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use dotenv_codegen::dotenv;
use reqwest::Client;
use log::trace;

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

fn http_client() -> ClientWithMiddleware {
  ClientBuilder::new(Client::new())
    .with(Cache(HttpCache {
      options: HttpCacheOptions::default(),
      manager: CACacheManager::default(),
      mode:    CacheMode::Default
    }))
    .build()
}

fn extract_message_from_response(response: &str) -> Result<String> {
  trace!("[extract_message_from_response] Response: {}", response);

  let response: Value = from_str(response).context("Failed to parse response")?;
  Ok(
    response["choices"]
      .as_array()
      .and_then(|choices| choices.get(0))
      .and_then(|choice| choice["message"]["content"].as_str())
      .context("Failed to extract commit message from response")?
      .to_string()
  )
}

// Fetches a commit message from the OpenAI API
async fn fetch_completion(payload: Value) -> Result<String> {
  trace!("[fetch_completion] Payload: {:?}", payload);

  http_client()
    .post(API_URL)
    .bearer_auth(API_KEY.as_str())
    .json(&payload)
    .send()
    .await.context("Failed to send request")?
    .text()
    .await
    .map_err(|e| e.into())
}

// Generates a commit message from the OpenAI API
pub async fn suggested_commit_message(diff: String) -> Result<String> {
  trace!("[suggested_commit_message] Generating commit message");

  let chat_prompt = build_commit_message_prompt("en", 72);
  let messages = vec![
    ChatMessage::new("system", chat_prompt),
    ChatMessage::new("user", diff),
  ];
  let payload = json_payload(messages);
  let response = fetch_completion(payload).await.context("Failed to fetch completion")?;
  Ok(extract_message_from_response(&response).context("Failed to extract message from response")?)
}
