//! Git diff parsing utilities.

use anyhow::Result;

/// Represents a parsed file from a git diff
#[derive(Debug, Clone)]
pub struct ParsedFile {
  pub path: String,
  pub operation: String,
  pub diff_content: String,
}

/// Extracts file path from diff header parts
///
/// Handles various git prefixes (a/, b/, c/, i/) and /dev/null for deleted files.
///
/// # Arguments
/// * `parts` - The whitespace-split parts from a "diff --git" line
///
/// # Returns
/// * `Option<String>` - The extracted path without prefixes, or None if parsing fails
fn extract_file_path_from_diff_parts(parts: &[&str]) -> Option<String> {
  if parts.len() < 4 {
    return None;
  }

  // Helper to strip git prefixes (a/, b/, c/, i/)
  let strip_prefix = |s: &str| {
    s.trim_start_matches("a/")
      .trim_start_matches("b/")
      .trim_start_matches("c/")
      .trim_start_matches("i/")
      .to_string()
  };

  let new_path = strip_prefix(parts[3]);
  let old_path = strip_prefix(parts[2]);

  // Prefer new path unless it's /dev/null (deleted file)
  Some(if new_path == "/dev/null" || new_path == "dev/null" {
    old_path
  } else {
    new_path
  })
}

/// Parse git diff into individual file changes.
///
/// Handles various diff formats including:
/// - Standard git diff output
/// - Diffs with commit hashes
/// - Diffs with various path prefixes (a/, b/, c/, i/)
/// - Deleted files (/dev/null paths)
///
/// # Arguments
/// * `diff_content` - Raw git diff text
///
/// # Returns
/// * `Result<Vec<ParsedFile>>` - Parsed files or error
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
      if let Some(path) = extract_file_path_from_diff_parts(&parts) {
        log::debug!("Found new file in diff: {path}");
        current_file = Some(ParsedFile {
          path,
          operation: "modified".to_string(), // Default, will be updated
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
        let mut found_path = false;

        // Safer approach: iterate through lines and find the path
        let mut extracted_path = String::new();
        for section_line in full_section.lines().take(3) {
          if section_line.starts_with("diff --git") {
            let parts: Vec<&str> = section_line.split_whitespace().collect();
            if let Some(p) = extract_file_path_from_diff_parts(&parts) {
              extracted_path = p;
              found_path = true;
              break;
            }
          }
        }

        if found_path {
          log::debug!("Found file in section {i}: {extracted_path}");
          files.push(ParsedFile {
            path:         extracted_path,
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

diff --git a/test.js b/test.js
new file mode 100644
index 0000000..a730e61
--- /dev/null
+++ b/test.js
@@ -0,0 +1 @@
+console.log('Hello');
"#;

    let files = parse_diff(diff).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "test.js");
    assert_eq!(files[0].operation, "added");
  }

  #[test]
  fn test_parse_diff_with_c_i_prefixes() {
    // Test with c/ and i/ prefixes that appear in git hook diffs
    let diff = r#"diff --git c/test.md i/test.md
new file mode 100644
index 0000000..6c61a60
--- /dev/null
+++ i/test.md
@@ -0,0 +1 @@
+# Test File

diff --git c/test.js i/test.js
new file mode 100644
index 0000000..a730e61
--- /dev/null
+++ i/test.js
@@ -0,0 +1 @@
+console.log('Hello');
"#;

    let files = parse_diff(diff).unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].path, "test.md", "Should extract clean path without c/ prefix");
    assert_eq!(files[0].operation, "added");
    assert_eq!(files[1].path, "test.js", "Should extract clean path without i/ prefix");
    assert_eq!(files[1].operation, "added");

    // Verify files contain diff content
    assert!(files[0].diff_content.contains("# Test File"));
    assert!(files[1].diff_content.contains("console.log"));
  }

  #[test]
  fn test_parse_diff_with_deleted_file() {
    let diff = r#"diff --git a/test.txt b/test.txt
deleted file mode 100644
index 9daeafb..0000000
--- a/test.txt
+++ /dev/null
@@ -1 +0,0 @@
-test
"#;

    let files = parse_diff(diff).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "test.txt");
    assert_eq!(files[0].operation, "deleted");
  }
}