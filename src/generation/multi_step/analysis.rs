//! File analysis for multi-step generation.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use async_openai::config::OpenAIConfig;
use async_openai::Client;
use serde_json::Value;

/// Represents a parsed file from the git diff
#[derive(Debug)]
pub struct ParsedFile {
  pub path:         String,
  pub operation:    String,
  pub diff_content: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysis {
  pub lines_added:   u32,
  pub lines_removed: u32,
  pub category:      FileCategory,
  pub summary:       String
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileCategory {
  Source,
  Test,
  Config,
  Docs,
  Binary,
  Build
}

impl FileCategory {
  pub fn as_str(&self) -> &'static str {
    match self {
      FileCategory::Source => "source",
      FileCategory::Test => "test",
      FileCategory::Config => "config",
      FileCategory::Docs => "docs",
      FileCategory::Binary => "binary",
      FileCategory::Build => "build"
    }
  }
}

impl From<&str> for FileCategory {
  fn from(s: &str) -> Self {
    match s {
      "source" => FileCategory::Source,
      "test" => FileCategory::Test,
      "config" => FileCategory::Config,
      "docs" => FileCategory::Docs,
      "binary" => FileCategory::Binary,
      "build" => FileCategory::Build,
      _ => FileCategory::Source // default fallback
    }
  }
}

/// Analyze a file locally without API
pub fn analyze_file(path: &str, diff_content: &str, operation: &str) -> FileAnalysis {
  // This will be moved from multi_step_analysis.rs analyze_file function
  crate::multi_step_analysis::analyze_file(path, diff_content, operation).into()
}

/// Analyze a file using OpenAI API
pub async fn analyze_file_via_api(
  client: &Client<OpenAIConfig>, model: &str, file: &crate::multi_step_integration::ParsedFile
) -> Result<Value> {
  // Delegate to the existing function for now
  crate::multi_step_integration::call_analyze_function(client, model, file).await
}

/// Helper: Categorize file by path
pub fn categorize_file(path: &str) -> FileCategory {
  // Implement locally for now to avoid private function call
  let path_lower = path.to_lowercase();

  if path_lower.ends_with("test.rs")
    || path_lower.ends_with("_test.rs")
    || path_lower.contains("tests/")
    || path_lower.ends_with(".test.js")
    || path_lower.ends_with(".spec.js")
  {
    FileCategory::Test
  } else if path_lower.ends_with(".md") || path_lower.ends_with(".rst") || path_lower.ends_with(".txt") {
    FileCategory::Docs
  } else if path_lower.ends_with("Cargo.toml")
    || path_lower.ends_with("package.json")
    || path_lower.ends_with("Makefile")
    || path_lower.ends_with("build.gradle")
    || path_lower.contains("cmake")
  {
    FileCategory::Build
  } else if path_lower.ends_with(".yml")
    || path_lower.ends_with(".yaml")
    || path_lower.ends_with(".json")
    || path_lower.ends_with(".toml")
    || path_lower.ends_with(".ini")
    || path_lower.ends_with(".cfg")
    || path_lower.ends_with(".conf")
    || path_lower.contains("config")
    || path_lower.contains(".github/")
  {
    FileCategory::Config
  } else if path_lower.ends_with(".png")
    || path_lower.ends_with(".jpg")
    || path_lower.ends_with(".gif")
    || path_lower.ends_with(".ico")
    || path_lower.ends_with(".pdf")
    || path_lower.ends_with(".zip")
  {
    FileCategory::Binary
  } else {
    FileCategory::Source
  }
}

// Conversion from old FileAnalysisResult to new FileAnalysis
impl From<crate::multi_step_analysis::FileAnalysisResult> for FileAnalysis {
  fn from(result: crate::multi_step_analysis::FileAnalysisResult) -> Self {
    FileAnalysis {
      lines_added:   result.lines_added,
      lines_removed: result.lines_removed,
      category:      FileCategory::from(result.file_category.as_str()),
      summary:       result.summary
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_file_categorization() {
    assert_eq!(categorize_file("src/main.rs"), FileCategory::Source);
    assert_eq!(categorize_file("tests/integration_test.rs"), FileCategory::Test);
    assert_eq!(categorize_file("package.json"), FileCategory::Build);
    assert_eq!(categorize_file(".github/workflows/ci.yml"), FileCategory::Config);
    assert_eq!(categorize_file("README.md"), FileCategory::Docs);
    assert_eq!(categorize_file("logo.png"), FileCategory::Binary);
  }
}
