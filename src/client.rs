use anyhow::{Context, Result};
use serde_json;

use crate::model::Model;
use crate::openai::{self, Request as OpenAIRequest};

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
  pub prompt:     String,
  pub system:     String,
  pub max_tokens: u16,
  pub model:      Model
}

#[derive(Debug, Clone, PartialEq)]
pub struct Response {
  pub response: String
}

pub async fn call(request: Request) -> Result<Response> {
  // Use the OpenAI client for all models
  let openai_request = OpenAIRequest {
    prompt:     request.prompt,
    system:     request.system,
    max_tokens: request.max_tokens,
    model:      request.model
  };

  let response = openai::call(openai_request).await?;
  Ok(Response { response: response.response })
}

pub async fn is_model_available(_model: Model) -> bool {
  true // OpenAI models are always considered available if API key is set
}
