use serde::{Deserialize, Serialize};
use serde_json::json;
use async_openai::types::{ChatCompletionTool, ChatCompletionToolType, FunctionObjectArgs};
use anyhow::Result;

/// Represents a file change in the commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
  /// Type of change: "added", "modified", "deleted", "renamed", or "binary"
  #[serde(rename = "type")]
  pub change_type:   String,
  /// Brief summary of changes to the file
  pub summary:       String,
  /// Total lines added + removed (0 for binary files)
  pub lines_changed: u32,
  /// Calculated impact score for prioritization (0.0 to 1.0)
  pub impact_score:  f32,
  /// File category for weighting calculations
  pub file_category: String
}

/// The commit function arguments structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitFunctionArgs {
  /// Justification for why the commit message is what it is
  pub reasoning: String,
  /// The commit message to be used
  pub message:   String,
  /// Hash of all altered files with their changes
  pub files:     std::collections::HashMap<String, FileChange>
}

/// Response from the commit function call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitFunctionCall {
  pub name:      String,
  pub arguments: String
}

/// Creates the commit function tool definition for OpenAI
pub fn create_commit_function_tool(max_length: Option<usize>) -> Result<ChatCompletionTool> {
  let max_length = max_length.unwrap_or(72);

  log::debug!("Creating commit function tool with max_length: {}", max_length);

  let function = FunctionObjectArgs::default()
        .name("commit")
        .description("Generate a git commit message based on the provided diff")
        .parameters(json!({
            "type": "object",
            "description": "The arguments for the commit function",
            "properties": {
                "reasoning": {
                    "type": "string",
                    "description": "Justification for why the commit message accurately represents the diff (1-2 sentences)",
                    "examples": [
                        "The diff shows a significant change in query logic from including specific values to excluding others, which fundamentally changes the filtering behavior",
                        "Multiple workflow files were updated to uncomment previously disabled steps, indicating a re-enabling of CI/CD processes",
                        "A new authentication module was added with comprehensive error handling, representing a significant feature addition",
                        "Authentication system implementation has highest impact (0.95) with 156 lines across core source files. Config changes (0.8 impact) support the feature."
                    ]
                },
                "message": {
                    "type": "string",
                    "description": "The actual commit message to be used",
                    "maxLength": max_length,
                    "examples": [
                        "Log failed AI jobs to Rollbar",
                        "Restore cronjob for AI tools",
                        "Add tmp/ as image dir",
                        "Test admin AI email",
                        "Improve auth 4xx errors in prompt",
                        "Disable security warning for fluentd",
                        "No whitespace between classes and modules",
                        "Update user authentication logic",
                        "Add input validation to login form",
                        "Fix bug in report generation",
                        "Refactor payment processing module",
                        "Remove deprecated API endpoints",
                        "Add JWT authentication system with middleware support"
                    ]
                },
                "files": {
                    "type": "object",
                    "description": "Object where keys are file paths and values describe the changes",
                    "additionalProperties": {
                        "type": "object",
                        "properties": {
                            "type": {
                                "type": "string",
                                "enum": ["added", "modified", "deleted", "renamed", "binary"],
                                "description": "The type of change made to the file"
                            },
                            "summary": {
                                "type": "string",
                                "description": "Brief summary of changes to the file",
                                "examples": [
                                    "Changed query from including unknown/nil/0 actions to excluding error/ignore actions",
                                    "Uncommented test execution steps",
                                    "New login functionality with validation",
                                    "Authentication-specific error types",
                                    "JWT token generation and validation functions",
                                    "Authentication middleware for protected routes"
                                ]
                            },
                            "lines_changed": {
                                "type": "integer",
                                "description": "Total lines added + removed (0 for binary files)",
                                "minimum": 0
                            },
                            "impact_score": {
                                "type": "number",
                                "minimum": 0.0,
                                "maximum": 1.0,
                                "description": "Calculated impact score for prioritization, 0.0 is lowest, 1.0 is highest"
                            },
                            "file_category": {
                                "type": "string",
                                "enum": ["source", "test", "config", "docs", "binary", "build"],
                                "description": "File category for weighting calculations"
                            }
                        },
                        "required": ["type", "summary", "lines_changed", "impact_score", "file_category"]
                    },
                    "examples": [
                        {
                            "app/jobs/invoice_analyzer_job.rb": {
                                "type": "modified",
                                "summary": "Changed query from including unknown/nil/0 actions to excluding error/ignore actions",
                                "lines_changed": 12,
                                "impact_score": 0.85,
                                "file_category": "source"
                            }
                        },
                        {
                            ".github/workflows/test.yml": {
                                "type": "modified",
                                "summary": "Uncommented test execution steps",
                                "lines_changed": 8,
                                "impact_score": 0.7,
                                "file_category": "config"
                            },
                            ".github/workflows/build.yml": {
                                "type": "modified",
                                "summary": "Uncommented build steps",
                                "lines_changed": 10,
                                "impact_score": 0.7,
                                "file_category": "config"
                            },
                            ".github/workflows/publish.yml": {
                                "type": "modified",
                                "summary": "Uncommented publish steps",
                                "lines_changed": 6,
                                "impact_score": 0.65,
                                "file_category": "config"
                            }
                        },
                        {
                            "src/auth/jwt.js": {
                                "type": "added",
                                "summary": "JWT token generation and validation functions",
                                "lines_changed": 89,
                                "impact_score": 0.95,
                                "file_category": "source"
                            },
                            "src/middleware/auth.js": {
                                "type": "added",
                                "summary": "Authentication middleware for protected routes",
                                "lines_changed": 67,
                                "impact_score": 0.85,
                                "file_category": "source"
                            },
                            "package.json": {
                                "type": "modified",
                                "summary": "Added jsonwebtoken and bcrypt dependencies",
                                "lines_changed": 3,
                                "impact_score": 0.8,
                                "file_category": "build"
                            },
                            "tests/auth.test.js": {
                                "type": "added",
                                "summary": "Unit tests for authentication functions",
                                "lines_changed": 45,
                                "impact_score": 0.6,
                                "file_category": "test"
                            },
                            "logo.png": {
                                "type": "modified",
                                "summary": "Updated company logo image",
                                "lines_changed": 0,
                                "impact_score": 0.1,
                                "file_category": "binary"
                            }
                        }
                    ]
                }
            },
            "required": ["reasoning", "message", "files"]
        }))
        .build()?;

  log::debug!("Successfully created commit function tool");
  log::trace!("Function definition: {:?}", function);

  Ok(ChatCompletionTool { r#type: ChatCompletionToolType::Function, function })
}

/// Parses the function call response to extract the commit message
pub fn parse_commit_function_response(arguments: &str) -> Result<CommitFunctionArgs> {
  log::debug!("Parsing commit function response");
  log::trace!("Raw arguments: {}", arguments);

  let args: CommitFunctionArgs = serde_json::from_str(arguments)?;

  // Log the reasoning and file changes in debug mode
  log::debug!("Commit reasoning: {}", args.reasoning);
  log::debug!("Commit message: '{}'", args.message);
  log::debug!("Message length: {} characters", args.message.len());

  log::debug!("Files changed: {} total", args.files.len());

  // Sort files by impact score for better debug output
  let mut sorted_files: Vec<(&String, &FileChange)> = args.files.iter().collect();
  sorted_files.sort_by(|a, b| b.1.impact_score.partial_cmp(&a.1.impact_score).unwrap());

  for (path, change) in sorted_files {
    log::debug!(
      "  {} ({}): {} [impact: {:.2}, lines: {}, category: {}]",
      path,
      change.change_type,
      change.summary,
      change.impact_score,
      change.lines_changed,
      change.file_category
    );
  }

  // Log summary statistics
  let total_lines: u32 = args.files.values().map(|f| f.lines_changed).sum();
  let avg_impact: f32 = args.files.values().map(|f| f.impact_score).sum::<f32>() / args.files.len() as f32;

  log::debug!("Summary statistics:");
  log::debug!("  Total lines changed: {}", total_lines);
  log::debug!("  Average impact score: {:.2}", avg_impact);

  // Count by file category
  let mut category_counts = std::collections::HashMap::new();
  for change in args.files.values() {
    *category_counts
      .entry(change.file_category.as_str())
      .or_insert(0) += 1;
  }

  log::debug!("  Files by category:");
  for (category, count) in category_counts {
    log::debug!("    {}: {}", category, count);
  }

  // Count by change type
  let mut type_counts = std::collections::HashMap::new();
  for change in args.files.values() {
    *type_counts.entry(change.change_type.as_str()).or_insert(0) += 1;
  }

  log::debug!("  Files by change type:");
  for (change_type, count) in type_counts {
    log::debug!("    {}: {}", change_type, count);
  }

  Ok(args)
}
