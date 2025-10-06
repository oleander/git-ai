use anyhow::Result;
use async_openai::config::OpenAIConfig;
use async_openai::types::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use async_openai::Client;
use serde_json::Value;
use futures::future::join_all;

use crate::multi_step_analysis::{
  create_analyze_function_tool, create_generate_function_tool, create_score_function_tool, FileDataForScoring, FileWithScore
};
use crate::function_calling::{create_commit_function_tool, CommitFunctionArgs};
use crate::debug_output;
use crate::diff::parser::{parse_diff, ParsedFile};

/// Main entry point for multi-step commit message generation
pub async fn generate_commit_message_multi_step(
  client: &Client<OpenAIConfig>, model: &str, diff_content: &str, max_length: Option<usize>
) -> Result<String> {
  log::info!("Starting multi-step commit message generation");

  // Initialize multi-step debug session
  if let Some(session) = debug_output::debug_session() {
    session.init_multi_step_debug();
  }

  // Parse the diff to extract individual files
  let parsed_files = parse_diff(diff_content)?;
  log::info!("Parsed {} files from diff", parsed_files.len());

  // Track files parsed in debug session
  if let Some(session) = debug_output::debug_session() {
    session.set_total_files_parsed(parsed_files.len());
  }

  // Step 1: Analyze each file individually in parallel
  log::debug!("Analyzing {} files in parallel", parsed_files.len());

  // Create futures for all file analyses
  let analysis_futures: Vec<_> = parsed_files
    .iter()
    .map(|file| {
      let file_path = file.path.clone();
      let operation = file.operation.clone();
      async move {
        log::debug!("Analyzing file: {file_path}");
        let start_time = std::time::Instant::now();
        let payload = format!("{{\"file_path\": \"{file_path}\", \"operation_type\": \"{operation}\", \"diff_content\": \"...\"}}");

        let result = call_analyze_function(client, model, file).await;
        let duration = start_time.elapsed();
        (file, result, duration, payload)
      }
    })
    .collect();

  // Execute all analyses in parallel
  let analysis_results = join_all(analysis_futures).await;

  // Process results and handle errors
  let mut file_analyses = Vec::new();
  for (i, (file, result, duration, payload)) in analysis_results.into_iter().enumerate() {
    match result {
      Ok(analysis) => {
        log::debug!("Successfully analyzed file {}: {}", i, file.path);

        // Extract structured analysis data for debug
        let analysis_result = crate::multi_step_analysis::FileAnalysisResult {
          lines_added:   analysis["lines_added"].as_u64().unwrap_or(0) as u32,
          lines_removed: analysis["lines_removed"].as_u64().unwrap_or(0) as u32,
          file_category: analysis["file_category"]
            .as_str()
            .unwrap_or("source")
            .to_string(),
          summary:       analysis["summary"].as_str().unwrap_or("").to_string()
        };

        // Record in debug session
        if let Some(session) = debug_output::debug_session() {
          session.add_file_analysis_debug(file.path.clone(), file.operation.clone(), analysis_result.clone(), duration, payload);
        }

        file_analyses.push((file, analysis));
      }
      Err(e) => {
        // Check if it's an API key error - if so, propagate it immediately
        let error_str = e.to_string();
        if error_str.contains("invalid_api_key") || error_str.contains("Incorrect API key") || error_str.contains("Invalid API key") {
          return Err(e);
        }
        log::warn!("Failed to analyze file {}: {}", file.path, e);
        // Continue with other files even if one fails
      }
    }
  }

  if file_analyses.is_empty() {
    anyhow::bail!("Failed to analyze any files");
  }

  // Step 2: Calculate impact scores
  let files_data: Vec<FileDataForScoring> = file_analyses
    .iter()
    .map(|(file, analysis)| {
      FileDataForScoring {
        file_path:      file.path.clone(),
        operation_type: file.operation.clone(),
        lines_added:    analysis["lines_added"].as_u64().unwrap_or(0) as u32,
        lines_removed:  analysis["lines_removed"].as_u64().unwrap_or(0) as u32,
        file_category:  analysis["file_category"]
          .as_str()
          .unwrap_or("source")
          .to_string(),
        summary:        analysis["summary"].as_str().unwrap_or("").to_string()
      }
    })
    .collect();

  // Record impact score calculation
  let score_start_time = std::time::Instant::now();
  let score_payload = format!(
    "{{\"files_data\": [{{\"{}\", ...}}, ...]}}",
    if !files_data.is_empty() {
      &files_data[0].file_path
    } else {
      "no files"
    }
  );

  // Start step 2 and 3 in parallel
  // First create the futures for both operations
  let score_future = call_score_function(client, model, files_data);

  // Run the scoring operation
  let scored_files = score_future.await?;
  let score_duration = score_start_time.elapsed();

  // Record in debug session
  if let Some(session) = debug_output::debug_session() {
    session.set_score_debug(scored_files.clone(), score_duration, score_payload);
  }

  // Step 3: Generate commit message candidates
  let generate_start_time = std::time::Instant::now();
  let generate_payload = format!("{{\"files_with_scores\": [...], \"max_length\": {}}}", max_length.unwrap_or(72));

  // Now create and run the generate and select steps in parallel
  let generate_future = call_generate_function(client, model, scored_files.clone(), max_length.unwrap_or(72));

  let candidates = generate_future.await?;
  let generate_duration = generate_start_time.elapsed();

  // Record in debug session
  if let Some(session) = debug_output::debug_session() {
    session.set_generate_debug(candidates.clone(), generate_duration, generate_payload);
  }

  // Step 4: Select the best candidate and format final response
  let final_message_start_time = std::time::Instant::now();
  let final_message = select_best_candidate(client, model, &candidates, &scored_files, diff_content).await?;
  let final_message_duration = final_message_start_time.elapsed();

  // Record in debug session
  if let Some(session) = debug_output::debug_session() {
    session.set_final_message_debug(final_message_duration);
    session.set_commit_result(final_message.clone(), candidates["reasoning"].as_str().unwrap_or("").to_string());
  }

  Ok(final_message)
}

/// Call the analyze function via OpenAI
async fn call_analyze_function(client: &Client<OpenAIConfig>, model: &str, file: &ParsedFile) -> Result<Value> {
  let tools = vec![create_analyze_function_tool()?];

  let system_message = ChatCompletionRequestSystemMessageArgs::default()
    .content("You are a git diff analyzer. Analyze the provided file changes and return structured data.")
    .build()?
    .into();

  let user_message = ChatCompletionRequestUserMessageArgs::default()
    .content(format!(
      "Analyze this file change:\nPath: {}\nOperation: {}\nDiff:\n{}",
      file.path, file.operation, file.diff_content
    ))
    .build()?
    .into();

  let request = CreateChatCompletionRequestArgs::default()
    .model(model)
    .messages(vec![system_message, user_message])
    .tools(tools)
    .tool_choice("analyze")
    .build()?;

  let response = client.chat().create(request).await?;

  if let Some(tool_call) = response.choices[0]
    .message
    .tool_calls
    .as_ref()
    .and_then(|calls| calls.first())
  {
    let args: Value = serde_json::from_str(&tool_call.function.arguments)?;
    Ok(args)
  } else {
    anyhow::bail!("No tool call in response")
  }
}

/// Call the score function via OpenAI
async fn call_score_function(
  client: &Client<OpenAIConfig>, model: &str, files_data: Vec<FileDataForScoring>
) -> Result<Vec<FileWithScore>> {
  let tools = vec![create_score_function_tool()?];

  let system_message = ChatCompletionRequestSystemMessageArgs::default()
    .content("You are a git commit impact scorer. Calculate impact scores for the provided file changes.")
    .build()?
    .into();

  let user_message = ChatCompletionRequestUserMessageArgs::default()
    .content(format!(
      "Calculate impact scores for these {} file changes:\n{}",
      files_data.len(),
      serde_json::to_string_pretty(&files_data)?
    ))
    .build()?
    .into();

  let request = CreateChatCompletionRequestArgs::default()
    .model(model)
    .messages(vec![system_message, user_message])
    .tools(tools)
    .tool_choice("score")
    .build()?;

  let response = client.chat().create(request).await?;

  if let Some(tool_call) = response.choices[0]
    .message
    .tool_calls
    .as_ref()
    .and_then(|calls| calls.first())
  {
    let args: Value = serde_json::from_str(&tool_call.function.arguments)?;
    let files_with_scores: Vec<FileWithScore> = if args["files_with_scores"].is_null() {
      Vec::new() // Return empty vector if null
    } else {
      serde_json::from_value(args["files_with_scores"].clone())?
    };
    Ok(files_with_scores)
  } else {
    anyhow::bail!("No tool call in response")
  }
}

/// Call the generate function via OpenAI
async fn call_generate_function(
  client: &Client<OpenAIConfig>, model: &str, files_with_scores: Vec<FileWithScore>, max_length: usize
) -> Result<Value> {
  let tools = vec![create_generate_function_tool()?];

  let system_message = ChatCompletionRequestSystemMessageArgs::default()
    .content("You are a git commit message generator. Generate concise, descriptive commit messages.")
    .build()?
    .into();

  let user_message = ChatCompletionRequestUserMessageArgs::default()
    .content(format!(
      "Generate commit message candidates (max {} chars) for these scored changes:\n{}",
      max_length,
      serde_json::to_string_pretty(&files_with_scores)?
    ))
    .build()?
    .into();

  let request = CreateChatCompletionRequestArgs::default()
    .model(model)
    .messages(vec![system_message, user_message])
    .tools(tools)
    .tool_choice("generate")
    .build()?;

  let response = client.chat().create(request).await?;

  if let Some(tool_call) = response.choices[0]
    .message
    .tool_calls
    .as_ref()
    .and_then(|calls| calls.first())
  {
    let args: Value = serde_json::from_str(&tool_call.function.arguments)?;
    Ok(args)
  } else {
    anyhow::bail!("No tool call in response")
  }
}

/// Select the best candidate and format the final response
async fn select_best_candidate(
  client: &Client<OpenAIConfig>, model: &str, candidates: &Value, scored_files: &[FileWithScore], original_diff: &str
) -> Result<String> {
  // Use the original commit function to get the final formatted response
  let tools = vec![create_commit_function_tool(Some(72))?];

  let system_message = ChatCompletionRequestSystemMessageArgs::default()
    .content(
      "You are a git commit message expert. Based on the multi-step analysis, \
            select the best commit message and provide the final formatted response."
    )
    .build()?
    .into();

  let user_message = ChatCompletionRequestUserMessageArgs::default()
    .content(format!(
      "Based on this multi-step analysis:\n\n\
            Candidates: {}\n\
            Reasoning: {}\n\n\
            Scored files: {}\n\n\
            Original diff:\n{}\n\n\
            Select the best commit message and format the response using the commit function.",
      serde_json::to_string_pretty(&candidates["candidates"])?,
      candidates["reasoning"].as_str().unwrap_or(""),
      serde_json::to_string_pretty(&scored_files)?,
      original_diff
    ))
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
    // First, parse as Value to manually handle required fields
    let raw_args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;

    // Extract the message which is what we really need
    if let Some(message) = raw_args.get("message").and_then(|m| m.as_str()) {
      return Ok(message.to_string());
    }

    // Fallback to full parsing if the above approach fails
    let args: CommitFunctionArgs = serde_json::from_str(&tool_call.function.arguments)?;
    Ok(args.message)
  } else {
    anyhow::bail!("No tool call in response")
  }
}

/// Alternative: Use the multi-step analysis locally without OpenAI calls
pub fn generate_commit_message_local(diff_content: &str, max_length: Option<usize>) -> Result<String> {
  use crate::multi_step_analysis::{analyze_file, calculate_impact_scores, generate_commit_messages};

  log::info!("Starting local multi-step commit message generation");

  // Parse the diff
  let parsed_files = parse_diff(diff_content)?;

  // Track files parsed in debug session
  if let Some(session) = debug_output::debug_session() {
    session.set_total_files_parsed(parsed_files.len());
  }

  // Step 1: Analyze each file
  let mut files_data = Vec::new();
  for file in parsed_files {
    let analysis = analyze_file(&file.path, &file.diff_content, &file.operation);
    files_data.push(FileDataForScoring {
      file_path:      file.path,
      operation_type: file.operation,
      lines_added:    analysis.lines_added,
      lines_removed:  analysis.lines_removed,
      file_category:  analysis.file_category,
      summary:        analysis.summary
    });
  }

  // Step 2: Calculate scores
  let score_result = calculate_impact_scores(files_data);

  // Step 3: Generate candidates
  let generate_result = generate_commit_messages(score_result.files_with_scores, max_length.unwrap_or(72));

  // Return the first candidate
  Ok(
    generate_result
      .candidates
      .first()
      .cloned()
      .unwrap_or_else(|| "Update files".to_string())
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_local_generation() {
    let diff = r#"diff --git a/src/auth.rs b/src/auth.rs
index 1234567..abcdefg 100644
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -10,7 +10,15 @@ pub fn authenticate(user: &str, pass: &str) -> Result<Token> {
-    if user == "admin" && pass == "password" {
-        Ok(Token::new())
-    } else {
-        Err(AuthError::InvalidCredentials)
-    }
+    // Validate input
+    if user.is_empty() || pass.is_empty() {
+        return Err(AuthError::EmptyCredentials);
+    }
+
+    // Check credentials against database
+    let hashed = hash_password(pass);
+    if validate_user(user, &hashed)? {
+        Ok(Token::generate(user))
+    } else {
+        Err(AuthError::InvalidCredentials)
+    }
 }"#;

    let message = generate_commit_message_local(diff, Some(72)).unwrap();
    assert!(!message.is_empty());
    assert!(message.len() <= 72);
  }
}
