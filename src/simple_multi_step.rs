use anyhow::Result;
use async_openai::config::OpenAIConfig;
use async_openai::types::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use async_openai::Client;

use crate::function_calling::{create_commit_function_tool, CommitFunctionArgs};
use crate::debug_output;

/// Simplified multi-step commit message generation that works with raw diff
pub async fn generate_commit_message_simple(
  client: &Client<OpenAIConfig>, model: &str, diff_content: &str, max_length: Option<usize>
) -> Result<String> {
  log::info!("Starting simplified multi-step commit message generation");

  // Initialize multi-step debug session
  if let Some(session) = debug_output::debug_session() {
    session.init_multi_step_debug();
  }

  // Use the commit function tool directly with the full diff
  let tools = vec![create_commit_function_tool(max_length)?];

  let system_message = ChatCompletionRequestSystemMessageArgs::default()
    .content(
      "You are a git commit message expert. Analyze the provided git diff and generate a concise, \
       descriptive commit message. Focus on the most significant changes and their impact. \
       The message should explain WHAT changed and WHY it matters."
    )
    .build()?
    .into();

  let user_message = ChatCompletionRequestUserMessageArgs::default()
    .content(format!("Generate a commit message for the following git diff:\n\n{}", diff_content))
    .build()?
    .into();

  let request = CreateChatCompletionRequestArgs::default()
    .model(model)
    .messages(vec![system_message, user_message])
    .tools(tools)
    .tool_choice("commit")
    .build()?;

  let response = client.chat().create(request).await?;

  if let Some(tool_call) = response.choices[0]
    .message
    .tool_calls
    .as_ref()
    .and_then(|calls| calls.first())
  {
    let args: CommitFunctionArgs = serde_json::from_str(&tool_call.function.arguments)?;

    // Record in debug session
    if let Some(session) = debug_output::debug_session() {
      session.set_commit_result(args.message.clone(), args.reasoning.clone());
      session.set_files_analyzed(args.clone());
      // Set a dummy count since we're not parsing files
      session.set_total_files_parsed(1);
    }

    Ok(args.message)
  } else {
    anyhow::bail!("No tool call in response")
  }
}

/// Local version that doesn't require parsing
pub fn generate_commit_message_simple_local(diff_content: &str, max_length: Option<usize>) -> Result<String> {
  log::info!("Starting simplified local commit message generation");

  // Count basic statistics from the diff
  let mut lines_added = 0;
  let mut lines_removed = 0;
  let mut files_mentioned = std::collections::HashSet::new();

  for line in diff_content.lines() {
    if line.starts_with("+++") || line.starts_with("---") {
      if let Some(file) = line.split_whitespace().nth(1) {
        files_mentioned.insert(file.trim_start_matches("a/").trim_start_matches("b/"));
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
      let file = files_mentioned.iter().next().unwrap();
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
        format!("Remove content from {}", file)
      } else {
        format!("Update {}", file)
      }
    }
    std::cmp::Ordering::Greater => format!("Update {} files", files_mentioned.len()),
    std::cmp::Ordering::Less => "Update files".to_string()
  };

  // Ensure it fits within the length limit
  let max_len = max_length.unwrap_or(72);
  if message.len() > max_len {
    Ok(message.chars().take(max_len - 3).collect::<String>() + "...")
  } else {
    Ok(message)
  }
}
