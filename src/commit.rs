use std::io;

use async_openai::types::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use async_openai::config::OpenAIConfig;
use async_openai::error::OpenAIError;
use async_openai::Client;
use thiserror::Error;
use anyhow::Context;
use anyhow::Result;

use crate::config;
use crate::model::Model;

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

fn instruction() -> String {
  format!("You are an AI assistant that generates concise and meaningful git commit messages based on provided diffs. Please adhere to the following guidelines:

  - Structure: Begin with a clear, present-tense summary.
  - Content: Emphasize the changes and their rationale, excluding irrelevant details.
  - Consistency: Maintain uniformity in tense, punctuation, and capitalization.
  - Accuracy: Ensure the message accurately reflects the changes and their purpose.
  - Present tense, imperative mood. (e.g., 'Add x to y' instead of 'Added x to y')
  - Max {} chars in the output

  ## Output:

  Your output should be a commit message generated from the input diff and nothing else.

  ## Input:

  INPUT:", config::APP.max_commit_length)
}

pub fn token_used(model: &Model) -> Result<usize> {
  model.count_tokens(&instruction()).context("Could not count tokens in instruction message")
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenAIResponse {
  pub response: String
}

pub async fn generate(diff: String) -> Result<OpenAIResponse, ChatError> {
  let api_key = config::APP
    .openai_api_key
    .clone()
    .context("Failed to get OpenAI API key, please run `git-ai config set openai-api")?;

  let config = OpenAIConfig::new().with_api_key(api_key);
  let client = Client::with_config(config);
  let request = CreateChatCompletionRequestArgs::default()
    .max_tokens(config::APP.max_tokens as u16)
    .model(config::APP.model.clone())
    .messages([
      ChatCompletionRequestSystemMessageArgs::default()
        .content(instruction())
        .build()?
        .into(),
      ChatCompletionRequestUserMessageArgs::default()
        .content(diff)
        .build()?
        .into()
    ])
    .build()?;

  let response = client.chat().create(request).await?;
  let reason = format!("Received empty response: {:?}", response);
  let choise = response.choices.first().context(reason)?;
  let text = choise.message.content.clone();

  Ok(OpenAIResponse { response: text.unwrap() })
}
