use std::time::{Duration, Instant};

use async_openai::types::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use async_openai::config::OpenAIConfig;
use async_openai::Client;
use async_openai::error::OpenAIError;
use anyhow::{anyhow, Context, Result};
use reqwest;
use futures::future::join_all;

use crate::{commit, config, debug_output, function_calling, profile};
use crate::model::Model;
use crate::config::AppConfig;
use crate::multi_step_integration::generate_commit_message_multi_step;

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
/// Now uses a simplified approach that doesn't require parsing the diff
pub async fn generate_commit_message(diff: &str) -> Result<String> {
  profile!("Generate commit message (simplified)");

  // Try to use the simplified approach with OpenAI
  if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
    if !api_key.is_empty() {
      // Use the commit function directly without parsing
      match commit::generate(diff.to_string(), 256, Model::GPT41Mini, None).await {
        Ok(response) => return Ok(response.response.trim().to_string()),
        Err(e) => {
          log::warn!("Direct generation failed, falling back to local: {e}");
        }
      }
    }
  }

  // Fallback to local generation (simplified version)
  // Count basic statistics from the diff
  let mut lines_added = 0;
  let mut lines_removed = 0;
  let mut files_mentioned = std::collections::HashSet::new();

  for line in diff.lines() {
    if line.starts_with("diff --git") {
      // Extract file path from diff --git line
      let parts: Vec<&str> = line.split_whitespace().collect();
      if parts.len() >= 4 {
        let path = parts[3].trim_start_matches("b/");
        files_mentioned.insert(path);
      }
    } else if line.starts_with("+++") || line.starts_with("---") {
      if let Some(file) = line.split_whitespace().nth(1) {
        let cleaned = file.trim_start_matches("a/").trim_start_matches("b/");
        if cleaned != "/dev/null" {
          files_mentioned.insert(cleaned);
        }
      }
    } else if line.starts_with('+') && !line.starts_with("+++") {
      lines_added += 1;
    } else if line.starts_with('-') && !line.starts_with("---") {
      lines_removed += 1;
    }
  }

  // Track in debug session
  if let Some(session) = debug_output::debug_session() {
    session.set_total_files_parsed(files_mentioned.len());
  }

  // Generate a simple commit message based on the diff
  let message = match files_mentioned.len().cmp(&1) {
    std::cmp::Ordering::Equal => {
      let file = files_mentioned
        .iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No files mentioned in commit message"))?;
      if lines_added > 0 && lines_removed == 0 {
        format!(
          "Add {} to {}",
          if lines_added == 1 {
            "content"
          } else {
            "new content"
          },
          file
        )
      } else if lines_removed > 0 && lines_added == 0 {
        format!("Remove content from {file}")
      } else {
        format!("Update {file}")
      }
    }
    std::cmp::Ordering::Greater => format!("Update {} files", files_mentioned.len()),
    std::cmp::Ordering::Less => "Update files".to_string()
  };

  Ok(message.trim().to_string())
}

/// Creates an OpenAI configuration from application settings
pub fn create_openai_config(settings: &AppConfig) -> Result<OpenAIConfig> {
  let api_key = settings
    .openai_api_key
    .as_ref()
    .ok_or_else(|| anyhow!("OpenAI API key not configured"))?;

  if api_key.is_empty() || api_key == "<PLACE HOLDER FOR YOUR API KEY>" {
    return Err(anyhow!("Invalid OpenAI API key"));
  }

  let config = OpenAIConfig::new().with_api_key(api_key);

  Ok(config)
}

/// Truncates text to fit within token limits
fn truncate_to_fit(text: &str, max_tokens: usize, model: &Model) -> Result<String> {
  profile!("Truncate to fit");

  // Fast path: if text is small, just return it
  if text.len() < 1000 {
    return Ok(text.to_string());
  }

  let token_count = model.count_tokens(text)?;
  if token_count <= max_tokens {
    return Ok(text.to_string());
  }

  // Collect character indices to ensure we slice at valid UTF-8 boundaries
  let char_indices: Vec<(usize, char)> = text.char_indices().collect();
  if char_indices.is_empty() {
    return Ok(String::new());
  }

  // Binary search for the right truncation point
  let mut low = 0;
  let mut high = char_indices.len();
  let mut best_fit = String::new();

  while low < high {
    let mid = (low + high) / 2;

    // Get the byte index for this character position
    let byte_index = if mid < char_indices.len() {
      char_indices[mid].0
    } else {
      text.len()
    };

    let truncated = &text[..byte_index];

    // Find the last complete line
    if let Some(last_newline_pos) = truncated.rfind('\n') {
      // Ensure we're at a valid UTF-8 boundary for the newline position
      let candidate = &text[..last_newline_pos];
      let candidate_tokens = model.count_tokens(candidate)?;

      if candidate_tokens <= max_tokens {
        best_fit = candidate.to_string();
        // Find the character index after the newline
        let next_char_idx = char_indices
          .iter()
          .position(|(idx, _)| *idx > last_newline_pos)
          .unwrap_or(char_indices.len());
        low = next_char_idx;
      } else {
        // Find the character index of the newline
        let newline_char_idx = char_indices
          .iter()
          .rposition(|(idx, _)| *idx <= last_newline_pos)
          .unwrap_or(0);
        high = newline_char_idx;
      }
    } else {
      high = mid;
    }
  }

  if best_fit.is_empty() {
    // If we couldn't find a good truncation point, just take what we can
    model.truncate(text, max_tokens)
  } else {
    Ok(best_fit)
  }
}

/// Generate with OpenAI using provided configuration
pub async fn generate_with_config(request: Request, config: OpenAIConfig) -> Result<Response> {
  profile!("OpenAI API call with custom config");

  // Always try multi-step approach first (it's now the default)
  let client = Client::with_config(config.clone());
  let model = request.model.to_string();

  match generate_commit_message_multi_step(&client, &model, &request.prompt, config::APP_CONFIG.max_commit_length).await {
    Ok(message) => return Ok(Response { response: message }),
    Err(e) => {
      // Check if it's an API key error and propagate it
      if e.to_string().contains("invalid_api_key") || e.to_string().contains("Incorrect API key") {
        return Err(e);
      }
      log::warn!("Multi-step approach failed, falling back to single-step: {e}");
    }
  }

  // Original single-step implementation as fallback
  // Create client with timeout if specified
  let client = if let Some(timeout) = config::APP_CONFIG.timeout {
    let http_client = reqwest::ClientBuilder::new()
      .timeout(Duration::from_secs(timeout as u64))
      .build()?;
    Client::with_config(config).with_http_client(http_client)
  } else {
    Client::with_config(config)
  };

  // Calculate available tokens using model's context size
  let system_tokens = request.model.count_tokens(&request.system)?;
  let model_context_size = request.model.context_size();
  let available_tokens = model_context_size.saturating_sub(system_tokens + request.max_tokens as usize);

  // Truncate prompt if needed
  let truncated_prompt = truncate_to_fit(&request.prompt, available_tokens, &request.model)?;

  // Create the commit function tool
  let commit_tool = function_calling::create_commit_function_tool(config::APP_CONFIG.max_commit_length)?;

  let chat_request = CreateChatCompletionRequestArgs::default()
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
    .tools(vec![commit_tool])
    .tool_choice("commit")
    .build()?;

  let mut last_error = None;

  for attempt in 1..=MAX_ATTEMPTS {
    log::debug!("OpenAI API attempt {attempt} of {MAX_ATTEMPTS}");

    // Track API call duration
    let api_start = Instant::now();

    match client.chat().create(chat_request.clone()).await {
      Ok(response) => {
        let api_duration = api_start.elapsed();

        // Record API duration in debug session
        if let Some(session) = debug_output::debug_session() {
          session.set_api_duration(api_duration);
        }

        log::debug!("OpenAI API call successful on attempt {attempt}");

        // Extract the response
        let choice = response
          .choices
          .into_iter()
          .next()
          .context("No response choices available")?;

        // Check if the model used function calling
        if let Some(tool_calls) = &choice.message.tool_calls {
          // Process multiple tool calls in parallel
          let tool_futures: Vec<_> = tool_calls
            .iter()
            .filter(|tool_call| tool_call.function.name == "commit")
            .map(|tool_call| {
              let args = tool_call.function.arguments.clone();
              async move { function_calling::parse_commit_function_response(&args) }
            })
            .collect();

          // Execute all tool calls in parallel
          let results = join_all(tool_futures).await;

          // Process results and handle errors
          let mut commit_messages = Vec::new();
          for (i, result) in results.into_iter().enumerate() {
            match result {
              Ok(commit_args) => {
                // Record commit results in debug session
                if let Some(session) = debug_output::debug_session() {
                  session.set_commit_result(commit_args.message.clone(), commit_args.reasoning.clone());
                  session.set_files_analyzed(commit_args.clone());
                }
                commit_messages.push(commit_args.message);
              }
              Err(e) => {
                log::warn!("Failed to parse tool call {i}: {e}");
              }
            }
          }

          // Return the first successful commit message or combine them if multiple
          if !commit_messages.is_empty() {
            // For now, return the first message. You could also combine them if needed
            return Ok(Response {
              response: commit_messages
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("No commit messages generated"))?
            });
          }
        }

        // Fallback to regular message content if no tool call
        let content = choice
          .message
          .content
          .clone()
          .context("No response content available")?;

        return Ok(Response { response: content });
      }
      Err(e) => {
        last_error = Some(e);
        log::warn!("OpenAI API attempt {attempt} failed");

        // Check if it's an API key error - fail immediately without retrying
        if let OpenAIError::ApiError(ref api_err) = &last_error.as_ref().unwrap() {
          if api_err.code.as_deref() == Some("invalid_api_key") {
            let error_msg = format!("Invalid OpenAI API key: {}", api_err.message);
            log::error!("{error_msg}");
            return Err(anyhow!(error_msg));
          }
        }

        if attempt < MAX_ATTEMPTS {
          let delay = Duration::from_millis(500 * attempt as u64);
          log::debug!("Retrying after {delay:?}");
          tokio::time::sleep(delay).await;
        }
      }
    }
  }

  // All attempts failed
  match last_error {
    Some(OpenAIError::ApiError(api_err)) => {
      let error_msg = format!(
        "OpenAI API error: {} (type: {:?}, code: {:?})",
        api_err.message,
        api_err.r#type.as_deref().unwrap_or("unknown"),
        api_err.code.as_deref().unwrap_or("unknown")
      );
      log::error!("{error_msg}");
      Err(anyhow!(error_msg))
    }
    Some(e) => {
      log::error!("OpenAI request failed: {e}");
      Err(anyhow!("OpenAI request failed: {}", e))
    }
    None => Err(anyhow!("OpenAI request failed after {} attempts", MAX_ATTEMPTS))
  }
}

/// Generate with OpenAI using default configuration from settings
pub async fn generate_with_openai(request: Request) -> Result<Response> {
  profile!("OpenAI API call");

  // Create OpenAI configuration using our settings
  let config = create_openai_config(&config::APP_CONFIG)?;

  // Use the generate_with_config function with the default config
  generate_with_config(request, config).await
}
