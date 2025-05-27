use serde::{Deserialize, Serialize};
use serde_json::json;
use async_openai::types::{ChatCompletionTool, ChatCompletionToolType, FunctionObjectArgs};
use anyhow::Result;

/// File analysis result from the analyze function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysisResult {
  pub lines_added:   u32,
  pub lines_removed: u32,
  pub file_category: String,
  pub summary:       String
}

/// File data with analysis results for scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDataForScoring {
  pub file_path:      String,
  pub operation_type: String,
  pub lines_added:    u32,
  pub lines_removed:  u32,
  pub file_category:  String,
  pub summary:        String
}

/// File data with calculated impact score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWithScore {
  pub file_path:      String,
  pub operation_type: String,
  pub lines_added:    u32,
  pub lines_removed:  u32,
  pub file_category:  String,
  pub summary:        String,
  pub impact_score:   f32
}

/// Score calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreResult {
  pub files_with_scores: Vec<FileWithScore>
}

/// Commit message generation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResult {
  pub candidates: Vec<String>,
  pub reasoning:  String
}

/// Creates the analyze function tool definition
pub fn create_analyze_function_tool() -> Result<ChatCompletionTool> {
  log::debug!("Creating analyze function tool");

  let function = FunctionObjectArgs::default()
    .name("analyze")
    .description("Analyze a single file's changes from the git diff")
    .parameters(json!({
        "type": "object",
        "properties": {
            "file_path": {
                "type": "string",
                "description": "Relative path to the file"
            },
            "diff_content": {
                "type": "string",
                "description": "The git diff content for this specific file only"
            },
            "operation_type": {
                "type": "string",
                "enum": ["added", "modified", "deleted", "renamed", "binary"],
                "description": "Type of operation performed on the file"
            }
        },
        "required": ["file_path", "diff_content", "operation_type"]
    }))
    .build()?;

  Ok(ChatCompletionTool { r#type: ChatCompletionToolType::Function, function })
}

/// Creates the score function tool definition
pub fn create_score_function_tool() -> Result<ChatCompletionTool> {
  log::debug!("Creating score function tool");

  let function = FunctionObjectArgs::default()
    .name("score")
    .description("Calculate impact scores for all analyzed files")
    .parameters(json!({
        "type": "object",
        "properties": {
            "files_data": {
                "type": "array",
                "description": "Array of analyzed file data",
                "items": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Relative path to the file"
                        },
                        "operation_type": {
                            "type": "string",
                            "enum": ["added", "modified", "deleted", "renamed", "binary"],
                            "description": "Type of operation performed on the file"
                        },
                        "lines_added": {
                            "type": "integer",
                            "description": "Number of lines added",
                            "minimum": 0
                        },
                        "lines_removed": {
                            "type": "integer",
                            "description": "Number of lines removed",
                            "minimum": 0
                        },
                        "file_category": {
                            "type": "string",
                            "enum": ["source", "test", "config", "docs", "binary", "build"],
                            "description": "Category of the file"
                        },
                        "summary": {
                            "type": "string",
                            "description": "Brief description of changes"
                        }
                    },
                    "required": ["file_path", "operation_type", "lines_added", "lines_removed", "file_category", "summary"]
                }
            }
        },
        "required": ["files_data"]
    }))
    .build()?;

  Ok(ChatCompletionTool { r#type: ChatCompletionToolType::Function, function })
}

/// Creates the generate function tool definition
pub fn create_generate_function_tool() -> Result<ChatCompletionTool> {
  log::debug!("Creating generate function tool");

  let function = FunctionObjectArgs::default()
    .name("generate")
    .description("Generate commit message candidates based on scored files")
    .parameters(json!({
        "type": "object",
        "properties": {
            "files_with_scores": {
                "type": "array",
                "description": "All files with calculated impact scores",
                "items": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string"
                        },
                        "operation_type": {
                            "type": "string"
                        },
                        "lines_added": {
                            "type": "integer"
                        },
                        "lines_removed": {
                            "type": "integer"
                        },
                        "file_category": {
                            "type": "string"
                        },
                        "summary": {
                            "type": "string"
                        },
                        "impact_score": {
                            "type": "number",
                            "minimum": 0.0,
                            "maximum": 1.0
                        }
                    },
                    "required": ["file_path", "operation_type", "lines_added", "lines_removed", "file_category", "summary", "impact_score"]
                }
            },
            "max_length": {
                "type": "integer",
                "description": "Maximum character length for commit message",
                "default": 72
            }
        },
        "required": ["files_with_scores"]
    }))
    .build()?;

  Ok(ChatCompletionTool { r#type: ChatCompletionToolType::Function, function })
}

/// Analyzes a single file's changes
pub fn analyze_file(file_path: &str, diff_content: &str, operation_type: &str) -> FileAnalysisResult {
  log::debug!("Analyzing file: {file_path} ({operation_type})");

  // Count lines added and removed
  let mut lines_added = 0u32;
  let mut lines_removed = 0u32;

  for line in diff_content.lines() {
    if line.starts_with('+') && !line.starts_with("+++") {
      lines_added += 1;
    } else if line.starts_with('-') && !line.starts_with("---") {
      lines_removed += 1;
    }
  }

  // Determine file category
  let file_category = categorize_file(file_path);

  // Generate summary based on diff content
  let summary = generate_file_summary(file_path, diff_content, operation_type);

  log::debug!("File analysis complete: +{lines_added} -{lines_removed} lines, category: {file_category}");

  FileAnalysisResult { lines_added, lines_removed, file_category, summary }
}

/// Calculates impact scores for all files
pub fn calculate_impact_scores(files_data: Vec<FileDataForScoring>) -> ScoreResult {
  log::debug!("Calculating impact scores for {} files", files_data.len());

  let mut files_with_scores = Vec::new();

  for file_data in files_data {
    let impact_score = calculate_single_impact_score(&file_data);

    files_with_scores.push(FileWithScore {
      file_path: file_data.file_path,
      operation_type: file_data.operation_type,
      lines_added: file_data.lines_added,
      lines_removed: file_data.lines_removed,
      file_category: file_data.file_category,
      summary: file_data.summary,
      impact_score
    });
  }

  // Sort by impact score descending
  files_with_scores.sort_by(|a, b| b.impact_score.partial_cmp(&a.impact_score).unwrap());

  ScoreResult { files_with_scores }
}

/// Generates commit message candidates
pub fn generate_commit_messages(files_with_scores: Vec<FileWithScore>, max_length: usize) -> GenerateResult {
  log::debug!("Generating commit messages (max length: {max_length})");

  // Find the highest impact changes
  let primary_change = files_with_scores.first();
  let mut candidates = Vec::new();

  if let Some(primary) = primary_change {
    // Generate different styles of commit messages

    // Style 1: Action-focused
    let action_msg = generate_action_message(primary, &files_with_scores, max_length);
    candidates.push(action_msg);

    // Style 2: Component-focused
    let component_msg = generate_component_message(primary, &files_with_scores, max_length);
    candidates.push(component_msg);

    // Style 3: Impact-focused
    let impact_msg = generate_impact_message(primary, &files_with_scores, max_length);
    candidates.push(impact_msg);
  }

  let reasoning = generate_reasoning(&files_with_scores);

  GenerateResult { candidates, reasoning }
}

// Helper functions

fn categorize_file(file_path: &str) -> String {
  let path = file_path.to_lowercase();

  if path.ends_with(".test.js")
    || path.ends_with(".spec.js")
    || path.ends_with("_test.go")
    || path.ends_with("_test.rs")
    || path.contains("/test/")
    || path.contains("/tests/")
  {
    "test".to_string()
  } else if path.ends_with(".md") || path.ends_with(".txt") || path.ends_with(".rst") || path.contains("/docs/") {
    "docs".to_string()
  } else if path == "package.json"
    || path == "cargo.toml"
    || path == "go.mod"
    || path == "requirements.txt"
    || path == "gemfile"
    || path.ends_with(".lock")
  {
    "build".to_string()
  } else if path.ends_with(".yml")
    || path.ends_with(".yaml")
    || path.ends_with(".json")
    || path.ends_with(".toml")
    || path.ends_with(".ini")
    || path.ends_with(".conf")
    || path.contains("config")
    || path.contains(".github/")
  {
    "config".to_string()
  } else if path.ends_with(".png")
    || path.ends_with(".jpg")
    || path.ends_with(".gif")
    || path.ends_with(".ico")
    || path.ends_with(".pdf")
    || path.ends_with(".zip")
  {
    "binary".to_string()
  } else {
    "source".to_string()
  }
}

fn generate_file_summary(file_path: &str, _diff_content: &str, operation_type: &str) -> String {
  // This is a simplified version - in practice, you'd analyze the diff content
  // more thoroughly to generate meaningful summaries
  match operation_type {
    "added" => format!("New {} file added", categorize_file(file_path)),
    "deleted" => format!("Removed {} file", categorize_file(file_path)),
    "renamed" => "File renamed".to_string(),
    "binary" => "Binary file updated".to_string(),
    _ => "File modified".to_string()
  }
}

fn calculate_single_impact_score(file_data: &FileDataForScoring) -> f32 {
  let mut score = 0.0f32;

  // Base score from operation type
  score += match file_data.operation_type.as_str() {
    "added" => 0.3,
    "modified" => 0.2,
    "deleted" => 0.25,
    "renamed" => 0.1,
    "binary" => 0.05,
    _ => 0.1
  };

  // Score from file category
  score += match file_data.file_category.as_str() {
    "source" => 0.4,
    "test" => 0.2,
    "config" => 0.25,
    "build" => 0.3,
    "docs" => 0.1,
    "binary" => 0.05,
    _ => 0.1
  };

  // Score from lines changed (normalized)
  let total_lines = file_data.lines_added + file_data.lines_removed;
  let line_score = (total_lines as f32 / 100.0).min(0.3);
  score += line_score;

  score.min(1.0) // Cap at 1.0
}

fn generate_action_message(primary: &FileWithScore, _all_files: &[FileWithScore], max_length: usize) -> String {
  let base = match primary.operation_type.as_str() {
    "added" => "Add",
    "modified" => "Update",
    "deleted" => "Remove",
    "renamed" => "Rename",
    _ => "Change"
  };

  let component = extract_component_name(&primary.file_path);
  let message = format!("{base} {component}");

  if message.len() > max_length {
    message.chars().take(max_length).collect()
  } else {
    message
  }
}

fn generate_component_message(primary: &FileWithScore, _all_files: &[FileWithScore], max_length: usize) -> String {
  let component = extract_component_name(&primary.file_path);
  let action = match primary.operation_type.as_str() {
    "added" => "implementation",
    "modified" => "updates",
    "deleted" => "removal",
    _ => "changes"
  };

  let message = format!("{component}: {action}");

  if message.len() > max_length {
    message.chars().take(max_length).collect()
  } else {
    message
  }
}

fn generate_impact_message(primary: &FileWithScore, all_files: &[FileWithScore], max_length: usize) -> String {
  let impact_type = if all_files
    .iter()
    .any(|f| f.file_category == "source" && f.operation_type == "added")
  {
    "feature"
  } else if all_files.iter().any(|f| f.file_category == "test") {
    "test"
  } else if all_files.iter().any(|f| f.file_category == "config") {
    "configuration"
  } else {
    "update"
  };

  let component = extract_component_name(&primary.file_path);
  let message = format!(
    "{} {} for {}",
    if impact_type == "feature" {
      "New"
    } else {
      "Update"
    },
    impact_type,
    component
  );

  if message.len() > max_length {
    message.chars().take(max_length).collect()
  } else {
    message
  }
}

fn extract_component_name(file_path: &str) -> String {
  let path_parts: Vec<&str> = file_path.split('/').collect();

  if let Some(filename) = path_parts.last() {
    // Remove extension
    let name_parts: Vec<&str> = filename.split('.').collect();
    if name_parts.len() > 1 {
      name_parts[0].to_string()
    } else {
      filename.to_string()
    }
  } else {
    "component".to_string()
  }
}

fn generate_reasoning(files_with_scores: &[FileWithScore]) -> String {
  if files_with_scores.is_empty() {
    return "No files to analyze".to_string();
  }

  let primary = &files_with_scores[0];
  let total_files = files_with_scores.len();
  let total_lines: u32 = files_with_scores
    .iter()
    .map(|f| f.lines_added + f.lines_removed)
    .sum();

  format!(
    "{} changes have highest impact ({:.2}) affecting {} functionality. \
        Total {} files changed with {} lines modified.",
    primary
      .file_category
      .chars()
      .next()
      .unwrap()
      .to_uppercase()
      .collect::<String>()
      + &primary.file_category[1..],
    primary.impact_score,
    extract_component_name(&primary.file_path),
    total_files,
    total_lines
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_file_categorization() {
    assert_eq!(categorize_file("src/main.rs"), "source");
    assert_eq!(categorize_file("tests/integration_test.rs"), "test");
    assert_eq!(categorize_file("package.json"), "build");
    assert_eq!(categorize_file(".github/workflows/ci.yml"), "config");
    assert_eq!(categorize_file("README.md"), "docs");
    assert_eq!(categorize_file("logo.png"), "binary");
  }

  #[test]
  fn test_impact_score_calculation() {
    let file_data = FileDataForScoring {
      file_path:      "src/auth.rs".to_string(),
      operation_type: "modified".to_string(),
      lines_added:    50,
      lines_removed:  20,
      file_category:  "source".to_string(),
      summary:        "Updated authentication logic".to_string()
    };

    let score = calculate_single_impact_score(&file_data);
    assert!(score > 0.0 && score <= 1.0);
  }
}
