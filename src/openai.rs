use async_openai::types::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use async_openai::config::OpenAIConfig;
use async_openai::Client;
use async_openai::error::OpenAIError;
use anyhow::{anyhow, Context, Result};
use colored::*;

use crate::{commit, config, profile};
use crate::model::Model;

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
pub async fn generate_commit_message(diff: &str) -> Result<String> {
  profile!("Generate commit message");
  let response = commit::generate(diff.into(), 256, Model::GPT4oMini).await?;
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
  if lines.is_empty() {
    return Ok(String::new());
  }

  // Try increasingly aggressive truncation until we fit
  for attempt in 0..MAX_ATTEMPTS {
    let keep_lines = match attempt {
      0 => lines.len() * 3 / 4, // First try: Keep 75%
      1 => lines.len() / 2,     // Second try: Keep 50%
      _ => lines.len() / 4      // Final try: Keep 25%
    };

    if keep_lines == 0 {
      break;
    }

    let mut truncated = Vec::new();
    truncated.extend(lines.iter().take(keep_lines));
    truncated.push("... (truncated for length) ...");

    let result = truncated.join("\n");
    let new_token_count = model.count_tokens(&result)?;

    if new_token_count <= max_tokens {
      return Ok(result);
    }
  }

  // If standard truncation failed, do minimal version with iterative reduction
  let mut minimal = Vec::new();
  let mut current_size = lines.len() / 50; // Start with 2% of lines

  while current_size > 0 {
    minimal.clear();
    minimal.extend(lines.iter().take(current_size));
    minimal.push("... (severely truncated for length) ...");

    let result = minimal.join("\n");
    let new_token_count = model.count_tokens(&result)?;

    if new_token_count <= max_tokens {
      return Ok(result);
    }

    current_size /= 2; // Halve the size each time
  }

  // If everything fails, return just the truncation message
  Ok("... (content too large, completely truncated) ...".to_string())
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

  // Calculate available tokens using model's context size
  let system_tokens = request.model.count_tokens(&request.system)?;
  let model_context_size = request.model.context_size();
  let available_tokens = model_context_size.saturating_sub(system_tokens + request.max_tokens as usize);

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
