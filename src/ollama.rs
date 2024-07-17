use anyhow::Result;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::options::GenerationOptions;
use ollama_rs::Ollama;

use crate::model::{Request, Response};

pub async fn call(request: Request) -> Result<Response> {
  let ollama = Ollama::default();

  let model = request.model.to_string();
  let prompt = format!("{}: {}", request.system, request.prompt);

  let options = GenerationOptions::default();

  let generation_request = GenerationRequest::new(model, prompt).options(options);

  let res = ollama.generate(generation_request).await?;

  Ok(Response { response: res.response })
}
