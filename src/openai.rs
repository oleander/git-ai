use std::time::{Duration, Instant};

use async_openai::types::chat::{
  ChatCompletionNamedToolChoice, ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, ChatCompletionToolChoiceOption, ChatCompletionTools, CreateChatCompletionRequestArgs
};
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

  let mut config = OpenAIConfig::new().with_api_key(api_key);

  // Allow pointing at a custom endpoint (e.g. a local ollama `/v1` server) when set.
  if let Some(base_url) = settings.openai_base_url.as_ref() {
    if !base_url.is_empty() {
      config = config.with_api_base(base_url);
    }
  }

  Ok(config)
}

/// Outcome of checking whether a model exists at the configured endpoint.
#[derive(Debug, PartialEq, Eq)]
pub enum ModelVerification {
  /// The model is usable (present in the listing, or a known/deprecated alias).
  Acceptable,
  /// The endpoint responded and the model is definitively absent.
  Absent
}

/// Pure decision logic for model verification, decoupled from any network IO so it
/// can be unit-tested with an injected list of available model ids.
///
/// * `candidate` - the model name the user is trying to set.
/// * `available_ids` - model ids returned by the endpoint's `/models` listing.
/// * `known_or_deprecated` - true if `candidate` maps to a built-in/deprecated model
///   (those are always acceptable regardless of what the endpoint advertises).
pub fn classify_model(candidate: &str, available_ids: &[String], known_or_deprecated: bool) -> ModelVerification {
  if known_or_deprecated {
    return ModelVerification::Acceptable;
  }

  let candidate = candidate.trim();
  if available_ids.iter().any(|id| id == candidate) {
    ModelVerification::Acceptable
  } else {
    ModelVerification::Absent
  }
}

/// Verifies that `candidate` exists at the configured endpoint before it is persisted.
///
/// Semantics (see F2):
/// * endpoint responds and model is present (or it's a known/deprecated alias) -> `Ok(())`.
/// * endpoint responds and model is definitively absent -> `Err(..)` (caller must not save).
/// * endpoint unreachable / unauthorized / no key configured (can't verify) -> `log::warn!`
///   and `Ok(())` so offline users are not hard-blocked.
pub async fn verify_model_exists(settings: &AppConfig, candidate: &str, known_or_deprecated: bool) -> Result<()> {
  // Known/deprecated aliases are always valid and need no round-trip.
  if known_or_deprecated {
    return Ok(());
  }

  // Without a usable key/config we cannot verify; warn and allow.
  let config = match create_openai_config(settings) {
    Ok(config) => config,
    Err(e) => {
      log::warn!("Could not verify model '{candidate}' (no usable OpenAI config: {e}); allowing it.");
      return Ok(());
    }
  };

  let client = Client::with_config(config);
  match client.models().list().await {
    Ok(list) => {
      let ids: Vec<String> = list.data.into_iter().map(|m| m.id).collect();
      match classify_model(candidate, &ids, known_or_deprecated) {
        ModelVerification::Acceptable => Ok(()),
        ModelVerification::Absent =>
          Err(anyhow!(
            "Model '{candidate}' is not available at the configured endpoint. \
           Run `git ai config set model <name>` with a model the endpoint offers."
          )),
      }
    }
    Err(e) => {
      log::warn!("Could not verify model '{candidate}' (endpoint unreachable/unauthorized: {e}); allowing it.");
      Ok(())
    }
  }
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

/// Calls the OpenAI API with the provided configuration
pub async fn call_with_config(request: Request, config: OpenAIConfig) -> Result<Response> {
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
    .max_completion_tokens(request.max_tokens as u32)
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
    .tools(vec![ChatCompletionTools::Function(commit_tool)])
    .tool_choice(ChatCompletionToolChoiceOption::Function(ChatCompletionNamedToolChoice::from("commit")))
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
            .filter_map(|tool_call| {
              match tool_call {
                async_openai::types::chat::ChatCompletionMessageToolCalls::Function(call) if call.function.name == "commit" =>
                  Some(call.function.arguments.clone()),
                _ => None
              }
            })
            .map(|args| async move { function_calling::parse_commit_function_response(&args) })
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
        if let Some(OpenAIError::ApiError(ref api_err)) = last_error.as_ref() {
          if api_err.api_error.code.as_deref() == Some("invalid_api_key") {
            let error_msg = format!("Invalid OpenAI API key: {}", api_err.api_error.message);
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
        api_err.api_error.message,
        api_err.api_error.r#type.as_deref().unwrap_or("unknown"),
        api_err.api_error.code.as_deref().unwrap_or("unknown")
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

/// Calls the OpenAI API with default configuration from settings
pub async fn call(request: Request) -> Result<Response> {
  profile!("OpenAI API call");

  // Create OpenAI configuration using our settings
  let config = create_openai_config(&config::APP_CONFIG)?;

  // Use the call_with_config function with the default config
  call_with_config(request, config).await
}

#[cfg(test)]
mod tests {
  use async_openai::config::{Config, OpenAIConfig};

  use super::*;

  fn settings_with(api_key: Option<&str>, base_url: Option<&str>) -> AppConfig {
    AppConfig {
      openai_api_key:    api_key.map(|s| s.to_string()),
      openai_base_url:   base_url.map(|s| s.to_string()),
      model:             Some("gpt-4.1-mini".to_string()),
      max_tokens:        Some(1024),
      max_commit_length: Some(72),
      timeout:           Some(30)
    }
  }

  /// F1: when no base URL is set, the config keeps async-openai's default api_base.
  #[test]
  fn test_create_openai_config_omits_base_when_absent() {
    let settings = settings_with(Some("sk-test-key"), None);
    let config = create_openai_config(&settings).unwrap();
    let default_base = OpenAIConfig::new().api_base().to_string();
    assert_eq!(config.api_base(), default_base);
  }

  /// F1: when a base URL is set, the config applies it via with_api_base.
  #[test]
  fn test_create_openai_config_applies_base_when_present() {
    let settings = settings_with(Some("sk-test-key"), Some("http://localhost:11434/v1"));
    let config = create_openai_config(&settings).unwrap();
    assert_eq!(config.api_base(), "http://localhost:11434/v1");
  }

  /// F1: an empty base URL string is treated as unset.
  #[test]
  fn test_create_openai_config_ignores_empty_base() {
    let settings = settings_with(Some("sk-test-key"), Some(""));
    let config = create_openai_config(&settings).unwrap();
    let default_base = OpenAIConfig::new().api_base().to_string();
    assert_eq!(config.api_base(), default_base);
  }

  /// F2: known/deprecated names are acceptable regardless of the endpoint listing.
  #[test]
  fn test_classify_model_known_is_acceptable() {
    assert_eq!(classify_model("gpt-4.1", &[], true), ModelVerification::Acceptable);
  }

  /// F2: an unknown model present in the endpoint listing is acceptable.
  #[test]
  fn test_classify_model_present_is_acceptable() {
    let ids = vec!["llama3.1:8b".to_string(), "mistral".to_string()];
    assert_eq!(classify_model("llama3.1:8b", &ids, false), ModelVerification::Acceptable);
  }

  /// F2: an unknown model definitively absent from the listing is Absent.
  #[test]
  fn test_classify_model_absent() {
    let ids = vec!["llama3.1:8b".to_string()];
    assert_eq!(classify_model("nonexistent-model", &ids, false), ModelVerification::Absent);
  }

  /// F2: when there is no usable config (no/placeholder key) we cannot verify, so allow.
  #[tokio::test]
  async fn test_verify_model_exists_allows_when_no_config() {
    let settings = settings_with(None, None);
    // Unknown model, but no key means we can't verify -> warn + allow (Ok).
    assert!(verify_model_exists(&settings, "some-model", false)
      .await
      .is_ok());
  }

  /// F2: known/deprecated names short-circuit without any network round-trip.
  #[tokio::test]
  async fn test_verify_model_exists_known_short_circuits() {
    let settings = settings_with(None, None);
    assert!(verify_model_exists(&settings, "gpt-4.1", true)
      .await
      .is_ok());
  }
}
