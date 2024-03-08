use std::{io, str};

use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::error::OpenAIError;
use thiserror::Error;
use anyhow::Context;
use async_openai::types::{
  ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage, ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs
};

use crate::config;

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
  #[error("Anyhow error: {0}")]
  Anyhow(#[from] anyhow::Error),
  #[error("OpenAI error: {0}")]
  OpenAIError(String),
  #[error("Failed to parse response: {1} ({0})")]
  ParseError(serde_json::Error, String),
  #[error("OpenAI error: {0}")]
  OpenAI(#[from] OpenAIError)
}

fn system_prompt(language: String, max_length_of_commit: usize) -> Result<ChatCompletionRequestSystemMessage, OpenAIError> {
  let payload = format!(
    "
    Your role is to create concise git commit messages based on user-provided git diffs. When crafting these messages:
    - Use {language}.
    - - Maximum Length: {max_length_of_commit} characters.
    - Focus on detailing the changes and reasons behind them, ensuring clarity and relevance.
    - Avoid including irrelevant or unnecessary details, such as translations, to maintain focus on the core changes.
    Your responses should be direct and immediately usable in a git commit, crafted in present tense to fit git conventions.
    You work primarily with git diffs, interpreting them to generate meaningful commit messages that succinctly summarize the changes.
  "
  )

  .split_whitespace()
  .collect::<Vec<&str>>()
  .join(" ");

  // TODO: Check out the options
  ChatCompletionRequestSystemMessageArgs::default().content(payload).build()
}

fn user_prompt(diff: String) -> Result<ChatCompletionRequestUserMessage, OpenAIError> {
  let payload = format!("Staged changes: {diff}").split_whitespace().collect::<Vec<&str>>().join(" ");

  ChatCompletionRequestUserMessageArgs::default().content(payload).build()
}

// Generate a commit message using OpenAI's API using the provided git diff
pub async fn generate(diff: String) -> Result<String, ChatError> {
  log::debug!("Generating commit message using config: {:?}", config::APP);

  let api_key = config::APP
    .openai_api_key
    .clone()
    .context("Failed to get OpenAI API key, please run `git-ai config set openapi-api-key <api-key>`")?;
  let max_length_of_commit = config::APP.max_length;
  let language = config::APP.language.clone();
  let model = config::APP.model.clone();

  let messages: Vec<ChatCompletionRequestMessage> =
    vec![system_prompt(language, max_length_of_commit)?.into(), user_prompt(diff)?.into()];

  log::debug!("Sending request to OpenAI API: {:?}", messages);

  let config = OpenAIConfig::new().with_api_key(api_key);
  let client = Client::with_config(config);

  log::debug!("Creating chat completion request");
  let request = CreateChatCompletionRequestArgs::default().messages(messages).model(model).n(1).build()?;

  log::debug!("Sending request to OpenAI API");
  client
    .chat()
    .create(request)
    .await?
    .choices
    .first()
    .and_then(|choice| choice.message.content.clone())
    .ok_or_else(|| ChatError::OpenAIError("Failed to get response from OpenAI".to_string()))
}
