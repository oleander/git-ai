use std::time::Duration;
use std::{io, str};

use async_openai::types::{
  AssistantObject, AssistantTools, AssistantToolsCode, CreateAssistantRequestArgs, CreateMessageRequestArgs, CreateRunRequestArgs, CreateThreadRequestArgs, MessageContent, RunStatus
};
use async_openai::config::OpenAIConfig;
use async_openai::error::OpenAIError;
use git2::Repository;
use indicatif::ProgressBar;
use async_openai::Client;
use tokio::time::sleep;
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

#[derive(Debug, Clone, PartialEq)]
pub struct Session {
  pub thread_id:    String,
  pub assistant_id: String
}

impl Session {
  pub async fn new_from_client(client: &Client<OpenAIConfig>) -> Result<Self, ChatError> {
    log::debug!("Creating new session from client");
    let assistant = create_assistant(client).await?;
    let thread_request = CreateThreadRequestArgs::default().build()?;
    let thread = client.threads().create(thread_request).await?;

    Ok(Session {
      thread_id: thread.id, assistant_id: assistant.id
    })
  }

  pub async fn load_from_repo(repo: &Repository) -> anyhow::Result<Option<Self>> {
    log::debug!("Loading session from repo");
    let config = repo.config().context("Failed to load config")?;
    let thread_id = config.get_string("ai.thread-id").ok();
    let assistant_id = config.get_string("ai.assistant-id").ok();
    log::debug!("Loaded session from repo: thread_id: {:?}, assistant_id: {:?}", thread_id, assistant_id);

    match (thread_id, assistant_id) {
      (Some(thread_id), Some(assistant_id)) => {
        Ok(Some(Session {
          thread_id,
          assistant_id
        }))
      },
      _ => Ok(None)
    }
  }

  // Save the session to the repository
  pub async fn save_to_repo(&self, repo: &Repository) -> anyhow::Result<()> {
    log::debug!("Saving session to repo");
    let mut config = repo.config().context("Failed to load config")?;

    config.set_str("ai.thread-id", self.thread_id.as_str())?;
    config.set_str("ai.assistant-id", self.assistant_id.as_str())?;
    config.snapshot().context("Failed to save config")?;
    Ok(())
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenAIResponse {
  pub session:  Session,
  pub response: String
}

async fn create_assistant(client: &Client<OpenAIConfig>) -> Result<AssistantObject, ChatError> {
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

#[derive(Debug, Clone)]
struct Connection {
  client:  Client<OpenAIConfig>,
  session: Session
}

impl Connection {
  pub async fn new(session: Option<Session>) -> Result<Self, ChatError> {
    let api_key = config::APP
      .openai_api_key
      .clone()
      .context("Failed to get OpenAI API key, please run `git-ai config set openapi-api")?;
    let config = OpenAIConfig::new().with_api_key(api_key);
    let client = Client::with_config(config);

    let session = match session {
      Some(session) => session,
      None => Session::new_from_client(&client).await?
    };

    Ok(Connection {
      client,
      session
    })
  }

  async fn create_run(&self) -> Result<Run, ChatError> {
    let request = CreateRunRequestArgs::default().assistant_id(self.session.clone().assistant_id).build()?;
    let run = self.client.threads().runs(&self.session.thread_id).create(request).await?;
    Ok(Run {
      id: run.id, connection: self.clone()
    })
  }

  async fn last_message(&self) -> Result<String, ChatError> {
    let query = [("limit", "1")];
    let response = self.client.threads().messages(&self.session.thread_id).list(&query).await?;
    let message_id = response.data.get(0).unwrap().id.clone();
    let message = self.client.threads().messages(&self.session.thread_id).retrieve(&message_id).await?;
    let content = message.content.get(0).unwrap();
    let MessageContent::Text(text) = &content else {
      return Err(ChatError::OpenAIError("Message content is not text".to_string()));
    };

    Ok(text.text.value.clone())
  }

  async fn create_message(&self, message: &str) -> Result<(), ChatError> {
    let message = CreateMessageRequestArgs::default().role("user").content(message).build()?;
    self.client.threads().messages(&self.session.thread_id).create(message).await?;
    Ok(())
  }

  async fn into_response(&self) -> Result<OpenAIResponse, ChatError> {
    let message = self.last_message().await?;
    let response = OpenAIResponse {
      response: message, session: self.session.clone()
    };
    Ok(response)
  }
}

#[derive(Debug, Clone)]
struct Run {
  id:         String,
  connection: Connection
}

impl Run {
  pub async fn pull_status(&self) -> Result<RunStatus, ChatError> {
    Ok(
      self
        .connection
        .client
        .threads()
        .runs(&self.connection.session.thread_id)
        .retrieve(self.id.as_str())
        .await?
        .status
    )
  }
}

pub async fn generate(
  diff: String, session: Option<Session>, progressbar: Option<ProgressBar>
) -> Result<OpenAIResponse, ChatError> {
  progressbar.clone().map(|pb| pb.set_message("Creating connection..."));

  let connection = Connection::new(session).await?;
  connection.create_message(&diff).await?;
  let run = connection.create_run().await?;

  let result = loop {
    match run.pull_status().await? {
      RunStatus::Completed => {
        break connection.into_response().await;
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
        progressbar.clone().map(|pb| pb.set_message("In progress..."));
      },
      RunStatus::Queued => {
        progressbar.clone().map(|pb| pb.set_message("Queued..."));
      },
      RunStatus::Cancelling => {
        progressbar.clone().map(|pb| pb.set_message("Cancelling..."));
      }
    }

    sleep(Duration::from_millis(300)).await;
  };

  result
}
