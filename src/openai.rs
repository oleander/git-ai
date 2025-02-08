use async_openai::types::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use async_openai::config::OpenAIConfig;
use async_openai::Client;
use async_openai::error::OpenAIError;
use anyhow::{anyhow, Context, Result};
use colored::*;

use crate::{config, profile};
use crate::model::Model;

const MAX_CONTEXT_LENGTH: usize = 128000;
const BUFFER_TOKENS: usize = 30000; // Large buffer for safety
const MAX_ATTEMPTS: usize = 3;

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

/// Generates an improved commit message using the provided prompt and diff
pub async fn generate_commit_message(diff: &str, prompt: &str, file_context: &str, author: &str, date: &str) -> Result<String> {
  profile!("Generate commit message");
  let system_prompt = format!(
    "You are an expert at writing clear, concise git commit messages. \
     Your task is to generate a commit message for the following code changes.\n\n\
     {}\n\n\
     Consider:\n\
     - Author: {}\n\
     - Date: {}\n\
     - Files changed: {}\n",
    prompt, author, date, file_context
  );

  let response = call(Request {
    system:     system_prompt,
    prompt:     format!("Generate a commit message for this diff:\n\n{}", diff),
    max_tokens: 256,
    model:      Model::GPT4oMini
  })
  .await?;

  Ok(response.response.trim().to_string())
}

/// Scores a commit message against the original using AI evaluation
pub async fn score_commit_message(message: &str, original: &str) -> Result<f32> {
  profile!("Score commit message");
  let system_prompt = "You are an expert at evaluating git commit messages. Score the following commit message on these criteria:
      - Accuracy (0-1): How well does it describe the actual changes?
      - Clarity (0-1): How clear and understandable is the message?
      - Brevity (0-1): Is it concise while being informative?
      - Categorization (0-1): Does it properly categorize the type of change?

      Return ONLY a JSON object containing these scores and brief feedback.";

  let response = call(Request {
    system:     system_prompt.to_string(),
    prompt:     format!("Original commit message:\n{}\n\nGenerated commit message:\n{}", original, message),
    max_tokens: 512,
    model:      Model::GPT4oMini
  })
  .await?;

  // Parse the JSON response to get the overall score
  let parsed: serde_json::Value = serde_json::from_str(&response.response).context("Failed to parse scoring response as JSON")?;

  let accuracy = parsed["accuracy"].as_f64().unwrap_or(0.0) as f32;
  let clarity = parsed["clarity"].as_f64().unwrap_or(0.0) as f32;
  let brevity = parsed["brevity"].as_f64().unwrap_or(0.0) as f32;
  let categorization = parsed["categorization"].as_f64().unwrap_or(0.0) as f32;

  Ok((accuracy + clarity + brevity + categorization) / 4.0)
}

/// Optimizes a prompt based on performance metrics
pub async fn optimize_prompt(current_prompt: &str, performance_metrics: &str) -> Result<String> {
  profile!("Optimize prompt");
  let system_prompt = "You are an expert at optimizing prompts for AI systems. \
      Your task is to improve a prompt used for generating git commit messages \
      based on performance metrics. Return ONLY the improved prompt text.";

  let response = call(Request {
    system:     system_prompt.to_string(),
    prompt:     format!(
      "Current prompt:\n{}\n\nPerformance metrics:\n{}\n\n\
       Suggest an improved version of this prompt that addresses any weaknesses \
       shown in the metrics while maintaining its strengths.",
      current_prompt, performance_metrics
    ),
    max_tokens: 1024,
    model:      Model::GPT4oMini
  })
  .await?;

  Ok(response.response.trim().to_string())
}

fn truncate_to_fit(text: &str, max_tokens: usize, model: &Model) -> Result<String> {
  let token_count = model.count_tokens(text)?;
  if token_count <= max_tokens {
    return Ok(text.to_string());
  }

  let lines: Vec<&str> = text.lines().collect();

  // Try increasingly aggressive truncation until we fit
  for attempt in 0..MAX_ATTEMPTS {
    let portion_size = match attempt {
      0 => lines.len() / 8,  // First try: Keep 25% (12.5% each end)
      1 => lines.len() / 12, // Second try: Keep ~16% (8% each end)
      _ => lines.len() / 20  // Final try: Keep 10% (5% each end)
    };

    let mut truncated = Vec::new();
    truncated.extend(lines.iter().take(portion_size));
    truncated.push("... (truncated for length) ...");
    truncated.extend(lines.iter().rev().take(portion_size).rev());

    let result = truncated.join("\n");
    let new_token_count = model.count_tokens(&result)?;

    if new_token_count <= max_tokens {
      return Ok(result);
    }
  }

  // If all attempts failed, return a minimal version
  let mut minimal = Vec::new();
  minimal.extend(lines.iter().take(lines.len() / 50));
  minimal.push("... (severely truncated for length) ...");
  minimal.extend(lines.iter().rev().take(lines.len() / 50).rev());
  Ok(minimal.join("\n"))
}

pub async fn call(request: Request) -> Result<Response> {
  profile!("OpenAI API call");
  let api_key = config::APP.openai_api_key.clone().context(format!(
    "{} OpenAI API key not found.\n    Run: {}",
    "ERROR:".bold().bright_red(),
    "git-ai config set openai-api-key <your-key>".yellow()
  ))?;

  let config = OpenAIConfig::new().with_api_key(api_key);
  let client = Client::with_config(config);

  // Calculate available tokens for content
  let system_tokens = request.model.count_tokens(&request.system)?;
  let available_tokens = MAX_CONTEXT_LENGTH.saturating_sub(system_tokens + BUFFER_TOKENS + request.max_tokens as usize);

  // Truncate prompt if needed
  let truncated_prompt = truncate_to_fit(&request.prompt, available_tokens, &request.model)?;

  let request = CreateChatCompletionRequestArgs::default()
    .max_tokens(request.max_tokens)
    .model(request.model.to_string())
    .messages([
      ChatCompletionRequestSystemMessageArgs::default()
        .content(request.system)
        .build()?
        .into(),
      ChatCompletionRequestUserMessageArgs::default()
        .content(truncated_prompt)
        .build()?
        .into()
    ])
    .build()?;

  {
    profile!("OpenAI request/response");
    let response = match client.chat().create(request).await {
      Ok(response) => response,
      Err(err) => {
        let error_msg = match err {
          OpenAIError::ApiError(e) =>
            format!(
              "{} {}\n    {}\n\nDetails:\n    {}\n\nSuggested Actions:\n    1. {}\n    2. {}\n    3. {}",
              "ERROR:".bold().bright_red(),
              "OpenAI API error:".bright_white(),
              e.message.dimmed(),
              "Failed to create chat completion.".dimmed(),
              "Ensure your OpenAI API key is valid".yellow(),
              "Check your account credits".yellow(),
              "Verify OpenAI service availability".yellow()
            ),
          OpenAIError::Reqwest(e) =>
            format!(
              "{} {}\n    {}\n\nDetails:\n    {}\n\nSuggested Actions:\n    1. {}\n    2. {}",
              "ERROR:".bold().bright_red(),
              "Network error:".bright_white(),
              e.to_string().dimmed(),
              "Failed to connect to OpenAI service.".dimmed(),
              "Check your internet connection".yellow(),
              "Verify OpenAI service is not experiencing downtime".yellow()
            ),
          _ =>
            format!(
              "{} {}\n    {}\n\nDetails:\n    {}",
              "ERROR:".bold().bright_red(),
              "Unexpected error:".bright_white(),
              err.to_string().dimmed(),
              "An unexpected error occurred while communicating with OpenAI.".dimmed()
            ),
        };
        return Err(anyhow!(error_msg));
      }
    };

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
