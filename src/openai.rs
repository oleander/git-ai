use async_openai::types::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use async_openai::config::OpenAIConfig;
use async_openai::Client;
use anyhow::{Context, Result};

use crate::{config, profile};
use crate::model::Model;

#[derive(Debug, Clone, PartialEq)]
pub struct Response {
  pub response: String
}

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
  pub prompt:     String,
  pub system:     String,
  pub max_tokens: u16,
  pub model:      Model
}

pub async fn call(request: Request) -> Result<Response> {
  profile!("OpenAI API call");
  let api_key = config::APP
    .openai_api_key
    .clone()
    .context("Failed to get OpenAI API key, please run `git-ai config set openai-api")?;

  let config = OpenAIConfig::new().with_api_key(api_key);
  let client = Client::with_config(config);

  let request = CreateChatCompletionRequestArgs::default()
    .max_tokens(request.max_tokens)
    .model(request.model.to_string())
    .messages([
      ChatCompletionRequestSystemMessageArgs::default()
        .content(request.system)
        .build()?
        .into(),
      ChatCompletionRequestUserMessageArgs::default()
        .content(request.prompt)
        .build()?
        .into()
    ])
    .build()?;

  {
    profile!("OpenAI request/response");
    let response = client
      .chat()
      .create(request)
      .await
      .context("Failed to create chat completion")?;

    let content = response
      .choices
      .first()
      .context("No choices returned")?
      .message
      .content
      .clone()
      .context("No content returned")?;

    Ok(Response { response: content })
  }
}
