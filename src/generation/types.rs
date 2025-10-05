use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
  pub file_path:      String,
  pub operation_type: String,
  pub diff_content:   Option<String>,
  pub lines_added:    u32,
  pub lines_removed:  u32,
  pub file_category:  String,
  pub summary:        String,
  pub impact_score:   f32
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OperationType {
  Added,
  Modified,
  Deleted,
  Renamed,
  Binary
}

impl OperationType {
  pub fn as_str(&self) -> &'static str {
    match self {
      OperationType::Added => "added",
      OperationType::Modified => "modified",
      OperationType::Deleted => "deleted",
      OperationType::Renamed => "renamed",
      OperationType::Binary => "binary"
    }
  }
}

impl From<&str> for OperationType {
  fn from(s: &str) -> Self {
    match s {
      "added" => OperationType::Added,
      "modified" => OperationType::Modified,
      "deleted" => OperationType::Deleted,
      "renamed" => OperationType::Renamed,
      "binary" => OperationType::Binary,
      _ => OperationType::Modified // default fallback
    }
  }
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

/// Unified response type for commit message generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResponse {
  pub message:   String,
  pub reasoning: String,
  pub files:     HashMap<String, FileChange>
}
