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

/// Represents a parsed file from the git diff
#[derive(Debug)]
pub struct ParsedFile {
  pub path:         String,
  pub operation:    String,
  pub diff_content: String
}

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

/// Parse git diff into individual files
pub fn parse_diff(diff_content: &str) -> Result<Vec<ParsedFile>> {
  let mut files = Vec::new();
  let mut current_file: Option<ParsedFile> = None;
  let mut current_diff = String::new();

  // Debug output
  log::debug!("Parsing diff with {} lines", diff_content.lines().count());

  // Add more detailed logging for debugging
  if log::log_enabled!(log::Level::Debug) && !diff_content.is_empty() {
    // Make sure we truncate at a valid UTF-8 character boundary
    let preview = if diff_content.len() > 500 {
      let truncated_index = diff_content
        .char_indices()
        .take_while(|(i, _)| *i < 500)
        .last()
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);

      format!("{}... (truncated)", &diff_content[..truncated_index])
    } else {
      diff_content.to_string()
    };
    log::debug!("Diff content preview: \n{preview}");
  }

  // Handle different diff formats
  let mut in_diff_section = false;
  let mut _commit_hash_line: Option<&str> = None;

  // First scan to detect if this is a commit message with hash
  for line in diff_content.lines().take(3) {
    if line.len() >= 40 && line.chars().take(40).all(|c| c.is_ascii_hexdigit()) {
      _commit_hash_line = Some(line);
      break;
    }
  }

  // Process line by line
  for line in diff_content.lines() {
    // Skip commit hash lines and other metadata
    if line.starts_with("commit ") || (line.len() >= 40 && line.chars().take(40).all(|c| c.is_ascii_hexdigit())) || line.is_empty() {
      continue;
    }

    // Check if we're starting a new file diff
    if line.starts_with("diff --git") {
      in_diff_section = true;
      // Save previous file if exists
      if let Some(mut file) = current_file.take() {
        file.diff_content = current_diff.clone();
        log::debug!("Adding file to results: {} ({})", file.path, file.operation);
        files.push(file);
        current_diff.clear();
      }

      // Extract file path more carefully
      let parts: Vec<&str> = line.split_whitespace().collect();
      if parts.len() >= 4 {
        let a_path = parts[2].trim_start_matches("a/");
        let b_path = parts[3].trim_start_matches("b/");

        // Use b_path (new) if available, otherwise use a_path (old)
        let path = if !b_path.is_empty() {
          b_path
        } else {
          a_path
        };
        log::debug!("Found new file in diff: {path}");
        current_file = Some(ParsedFile {
          path:         path.to_string(),
          operation:    "modified".to_string(), // Default, will be updated
          diff_content: String::new()
        });
      }

      // Add the header line to the diff content
      current_diff.push_str(line);
      current_diff.push('\n');
    } else if line.starts_with("new file mode") {
      if let Some(ref mut file) = current_file {
        log::debug!("File {} is newly added", file.path);
        file.operation = "added".to_string();
      }
      current_diff.push_str(line);
      current_diff.push('\n');
    } else if line.starts_with("deleted file mode") {
      if let Some(ref mut file) = current_file {
        log::debug!("File {} is deleted", file.path);
        file.operation = "deleted".to_string();
      }
      current_diff.push_str(line);
      current_diff.push('\n');
    } else if line.starts_with("rename from") || line.starts_with("rename to") {
      if let Some(ref mut file) = current_file {
        log::debug!("File {} is renamed", file.path);
        file.operation = "renamed".to_string();
      }
      current_diff.push_str(line);
      current_diff.push('\n');
    } else if line.starts_with("Binary files") {
      if let Some(ref mut file) = current_file {
        log::debug!("File {} is binary", file.path);
        file.operation = "binary".to_string();
      }
      current_diff.push_str(line);
      current_diff.push('\n');
    } else if line.starts_with("index ") || line.starts_with("--- ") || line.starts_with("+++ ") || line.starts_with("@@ ") {
      // These are important diff headers that should be included
      current_diff.push_str(line);
      current_diff.push('\n');
    } else if in_diff_section {
      current_diff.push_str(line);
      current_diff.push('\n');
    }
  }

  // Don't forget the last file
  if let Some(mut file) = current_file {
    file.diff_content = current_diff;
    log::debug!("Adding final file to results: {} ({})", file.path, file.operation);
    files.push(file);
  }

  // If we didn't parse any files, check if this looks like a raw git diff output
  // from commands like `git show` that include commit info at the top
  if files.is_empty() && !diff_content.trim().is_empty() {
    log::debug!("Trying to parse as raw git diff output with commit info");

    // Extract sections that start with "diff --git"
    let sections: Vec<&str> = diff_content.split("diff --git").skip(1).collect();

    if !sections.is_empty() {
      for (i, section) in sections.iter().enumerate() {
        // Add the "diff --git" prefix back
        let full_section = format!("diff --git{section}");

        // Extract file path from the section more carefully
        let mut path = "unknown";
        let mut found_path = false;

        // Safer approach: iterate through lines and find the path
        for section_line in full_section.lines().take(3) {
          if section_line.starts_with("diff --git") {
            let parts: Vec<&str> = section_line.split_whitespace().collect();
            if parts.len() >= 4 {
              path = parts[3].trim_start_matches("b/");
              found_path = true;
              break;
            }
          }
        }

        if found_path {
          log::debug!("Found file in section {i}: {path}");
          files.push(ParsedFile {
            path:         path.to_string(),
            operation:    "modified".to_string(), // Default
            diff_content: full_section
          });
        }
      }
    }
  }

  // If still no files were parsed, treat the entire diff as a single change
  if files.is_empty() && !diff_content.trim().is_empty() {
    log::debug!("No standard diff format found, treating as single file change");
    files.push(ParsedFile {
      path:         "unknown".to_string(),
      operation:    "modified".to_string(),
      diff_content: diff_content.to_string()
    });
  }

  log::debug!("Parsed {} files from diff", files.len());

  // Add detailed debug output for each parsed file
  if log::log_enabled!(log::Level::Debug) {
    for (i, file) in files.iter().enumerate() {
      let content_preview = if file.diff_content.len() > 200 {
        // Make sure we truncate at a valid UTF-8 character boundary
        let truncated_index = file
          .diff_content
          .char_indices()
          .take_while(|(i, _)| *i < 200)
          .last()
          .map(|(i, c)| i + c.len_utf8())
          .unwrap_or(0);

        format!("{}... (truncated)", &file.diff_content[..truncated_index])
      } else {
        file.diff_content.clone()
      };
      log::debug!("File {}: {} ({})\nContent preview:\n{}", i, file.path, file.operation, content_preview);
    }
  }

  Ok(files)
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
  fn test_parse_diff() {
    let diff = r#"diff --git a/src/main.rs b/src/main.rs
index 1234567..abcdefg 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,6 @@
 fn main() {
-    println!("Hello");
+    println!("Hello, world!");
+    println!("New line");
 }
diff --git a/Cargo.toml b/Cargo.toml
new file mode 100644
index 0000000..1111111
--- /dev/null
+++ b/Cargo.toml
@@ -0,0 +1,8 @@
+[package]
+name = "test"
+version = "0.1.0"
"#;

    let files = parse_diff(diff).unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].path, "src/main.rs");
    assert_eq!(files[0].operation, "modified");
    assert_eq!(files[1].path, "Cargo.toml");
    assert_eq!(files[1].operation, "added");

    // Verify files contain diff content
    assert!(!files[0].diff_content.is_empty());
    assert!(!files[1].diff_content.is_empty());
  }

  #[test]
  fn test_parse_diff_with_commit_hash() {
    // Test with a commit hash and message before the diff
    let diff = r#"0472ffa1665c4c5573fb8f7698c9965122eda675 Update files
diff --git a/src/openai.rs b/src/openai.rs
index a67ebbe..da223be 100644
--- a/src/openai.rs
+++ b/src/openai.rs
@@ -15,11 +15,6 @@ use crate::multi_step_integration::{generate_commit_message_local, generate_comm

 const MAX_ATTEMPTS: usize = 3;

-#[derive(Debug, Clone, PartialEq)]
-pub struct Response {
-  pub response: String
-}
-
 #[derive(Debug, Clone, PartialEq)]
 pub struct Request {
   pub prompt:     String,
@@ -28,6 +23,11 @@ pub struct Request {
   pub model:      Model
 }

+#[derive(Debug, Clone, PartialEq)]
+pub struct Response {
+  pub response: String
+}
+
 /// Generates an improved commit message using the provided prompt and diff
 /// Now uses the multi-step approach by default
 pub async fn generate_commit_message(diff: &str) -> Result<String> {
"#;

    let files = parse_diff(diff).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "src/openai.rs");
    assert_eq!(files[0].operation, "modified");

    // Verify diff content contains actual changes
    assert!(files[0].diff_content.contains("pub struct Response"));

    // Verify commit hash line was skipped
    assert!(!files[0]
      .diff_content
      .contains("0472ffa1665c4c5573fb8f7698c9965122eda675"));
  }

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
