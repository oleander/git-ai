use async_openai::types::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use async_openai::config::OpenAIConfig;
use async_openai::Client;
use anyhow::{Context, Result};

use crate::config;
use crate::model::Response;
use crate::model::Request;


pub async fn call(request: Request) -> Result<Response> {
  let api_key = config::APP
    .openai_api_key
    .clone()
    .context("Failed to get OpenAI API key, please run `git-ai config set openai-api")?;

  let config = OpenAIConfig::new().with_api_key(api_key);
  let client = Client::with_config(config);

  let request = CreateChatCompletionRequestArgs::default()
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

  let chat = client.chat().create(request).await?;

  let choise = chat
    .choices
    .first()
    .context(format!("Failed to get response: {:?}", chat))?;

  let response = choise
    .message
    .content
    .clone()
    .context("Failed to get response text")?;

  Ok(Response { response })
}
