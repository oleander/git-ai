use std::time::Duration;
use std::{io, str};

use async_openai::types::{
  AssistantObject, AssistantTools, AssistantToolsCode, CreateAssistantRequestArgs, CreateMessageRequestArgs, CreateRunRequestArgs, CreateThreadRequestArgs, MessageContent, RunStatus
};
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::error::OpenAIError;
use thiserror::Error;
use anyhow::Context;
use tokio::time::sleep;

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
  format!("Staged changes: {diff}")
    .split_whitespace()
    .collect::<Vec<&str>>()
    .join(" ")
}

fn client() -> Result<Client<OpenAIConfig>, ChatError> {
  let api_key = config::APP
    .openai_api_key
    .clone()
    .context("Failed to get OpenAI API key, please run `git-ai config set openapi-api-key <api-key>`")?;

  let config = OpenAIConfig::new().with_api_key(api_key);
  Ok(Client::with_config(config))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Session {
  pub thread_id:    String,
  pub assistant_id: String
}

impl Session {
  pub async fn new_from_client(client: &Client<OpenAIConfig>) -> Result<Self, ChatError> {
    let assistant = create_assistant(client).await?;
    let thread_request = CreateThreadRequestArgs::default().build()?;
    let thread = client.threads().create(thread_request).await?;

    Ok(Session {
      thread_id: thread.id, assistant_id: assistant.id
    })
  }
}

pub struct OpenAIResponse {
  pub session:  Session,
  pub response: String
}

async fn create_assistant(
  client: &Client<OpenAIConfig>
) -> Result<AssistantObject, ChatError> {
  let language = config::APP.language.clone();
  let max_length_of_commit = config::APP.max_length;
  let model = config::APP.model.clone();
  let instruction = instruction(language, max_length_of_commit);

  let tools = vec![AssistantTools::Code(AssistantToolsCode {
    r#type: "code_interpreter".to_string()
  })];

  let assistant_request = CreateAssistantRequestArgs::default()
    .name("Git Commit Assistant")
    .instructions(&instruction)
    .tools(tools)
    .model(model)
    .build()?;

  Ok(client.assistants().create(assistant_request).await?)
}

// Generate a commit message using OpenAI's API using the provided git diff
pub async fn generate(
  diff: String, session: Option<Session>
) -> Result<OpenAIResponse, ChatError> {
  let query = [("limit", "1")];
  let client = client()?;

  let session = match session {
    Some(session) => session,
    None => Session::new_from_client(&client).await?
  };

  let thread_id = session.clone().thread_id;
  let assistant_id = session.clone().assistant_id;

  let message = CreateMessageRequestArgs::default()
    .role("user")
    .content(user_prompt(diff))
    .build()?;

  client.threads().messages(&thread_id).create(message).await?;

  let run_request = CreateRunRequestArgs::default().assistant_id(assistant_id).build()?;
  let run = client.threads().runs(&thread_id).create(run_request).await?;

  let result = loop {
    let run = client.threads().runs(&thread_id).retrieve(&run.id).await?;
    match run.status {
      RunStatus::Completed => {
        let response = client.threads().messages(&thread_id).list(&query).await?;
        let message_id = response.data.get(0).unwrap().id.clone();
        let message = client.threads().messages(&thread_id).retrieve(&message_id).await?;
        let content = message.content.get(0).unwrap();

        let MessageContent::Text(text) = &content else {
          break Err(ChatError::OpenAIError("Message content is not text".to_string()));
        };

        break Ok(text.text.value.clone());
      },
      RunStatus::Failed => {
        break Err(ChatError::OpenAIError("Run failed".to_string()));
      },
      RunStatus::Cancelled => {
        break Err(ChatError::OpenAIError("Run cancelled".to_string()));
      },
      RunStatus::Expired => {
        break Err(ChatError::OpenAIError("Run expired".to_string()));
      },
      RunStatus::RequiresAction => {
        break Err(ChatError::OpenAIError("Run requires action".to_string()));
      },
      RunStatus::InProgress => {
        log::debug!("--- Run InProgress");
      },
      RunStatus::Queued => {
        log::debug!("--- Run Queued");
      },
      RunStatus::Cancelling => {
        log::debug!("--- Run Cancelling");
      }
    }

    sleep(Duration::from_millis(300)).await;
  };

  Ok(OpenAIResponse {
    response: result?,
    session
  })
}
