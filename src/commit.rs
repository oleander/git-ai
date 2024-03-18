use std::{io, str};

use async_openai::types::{
  CreateAssistantRequestArgs, CreateMessageRequestArgs, CreateRunRequestArgs, CreateThreadRequestArgs, MessageContent, RunStatus
};
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::error::OpenAIError;
use thiserror::Error;
use anyhow::Context;

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

fn instruction(language: String, max_length_of_commit: usize) -> String {
  format!(
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
  .join(" ")
}

fn user_prompt(diff: String) -> String {
  format!("Staged changes: {diff}").split_whitespace().collect::<Vec<&str>>().join(" ")
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

  let config = OpenAIConfig::new().with_api_key(api_key);
  let client = Client::with_config(config);
  let query = [("limit", "1")];
  let thread_request = CreateThreadRequestArgs::default().build()?;
  let thread = client.threads().create(thread_request.clone()).await?;
  let instruction = instruction(language, max_length_of_commit);
  let assistant_request = CreateAssistantRequestArgs::default()
    .name("Git Commit Assistant")
    .instructions(&instruction)
    .model(model)
    .build()?;

  let assistant = client.assistants().create(assistant_request).await?;
  let assistant_id = &assistant.id;
  let message = CreateMessageRequestArgs::default().role("user").content(user_prompt(diff)).build()?;

  //attach message to the thread
  let _message_obj = client.threads().messages(&thread.id).create(message).await?;

  let run_request = CreateRunRequestArgs::default().assistant_id(assistant_id).build()?;

  let run = client.threads().runs(&thread.id).create(run_request).await?;

  let result = loop {
    let run = client.threads().runs(&thread.id).retrieve(&run.id).await?;
    match run.status {
      RunStatus::Completed => {
        let response = client.threads().messages(&thread.id).list(&query).await?;
        let message_id = response.data.get(0).unwrap().id.clone();
        let message = client.threads().messages(&thread.id).retrieve(&message_id).await?;
        let content = message.content.get(0).unwrap();
        let text = match content {
          MessageContent::Text(text) => text.text.value.clone(),
          MessageContent::ImageFile(_) => {
            panic!("imaged are not supported in the terminal")
          }
        };

        break Ok(text);
      },
      RunStatus::Failed => {
        println!("--- Run Failed: {:#?}", run);
        break Err(ChatError::OpenAIError("Run failed".to_string()));
      },
      RunStatus::Queued => {
        println!("--- Run Queued");
      },
      RunStatus::Cancelling => {
        println!("--- Run Cancelling");
      },
      RunStatus::Cancelled => {
        println!("--- Run Cancelled");
      },
      RunStatus::Expired => {
        println!("--- Run Expired");
      },
      RunStatus::RequiresAction => {
        println!("--- Run Requires Action");
      },
      RunStatus::InProgress => {
        println!("--- Waiting for response...");
      }
    }
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
  };

  client.assistants().delete(assistant_id).await?;
  client.threads().delete(&thread.id).await?;

  result
}
