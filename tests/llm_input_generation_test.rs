use ai::commit::{calculate_token_usage, create_commit_request, generate_instruction_template};
use ai::model::Model;

/// Tests for the LLM input generation system
/// These tests verify that the prompt templates and request creation work correctly
/// with various edge cases and scenarios.

// =============================================================================
// SECTION 1: Minimal/Corner Case Tests
// =============================================================================

#[test]
fn test_template_generation_with_default_max_length() {
  // Test that template generation works with default config
  let result = generate_instruction_template();
  assert!(result.is_ok(), "Template generation should succeed");

  let template = result.unwrap();
  assert!(!template.is_empty(), "Template should not be empty");
  assert!(template.contains("Git Commit Message"), "Template should contain header");
  assert!(template.contains("max_length"), "Template should contain max_length reference");
}

// Note: Testing custom max_length would require modifying global config
// which is unsafe and not a good testing practice. Instead, we verify
// that the template rendering system works correctly with the default config.

#[test]
fn test_token_counting_empty_template() {
  // Token counting should work even with minimal content
  let model = Model::GPT41Mini;
  let result = model.count_tokens("");
  assert!(result.is_ok(), "Should handle empty string");
  assert_eq!(result.unwrap(), 0, "Empty string should have 0 tokens");
}

#[test]
fn test_token_counting_template() {
  // Test that we can count tokens in the actual template
  let model = Model::GPT41Mini;
  let result = calculate_token_usage(&model);

  assert!(result.is_ok(), "Token counting should succeed");
  let token_count = result.unwrap();

  // The template is substantial, should have a reasonable token count
  assert!(token_count > 100, "Template should have at least 100 tokens, got {}", token_count);
  assert!(token_count < 10000, "Template shouldn't exceed 10000 tokens, got {}", token_count);
}

#[test]
fn test_create_request_with_zero_tokens() {
  // Edge case: what happens with 0 max_tokens?
  let diff = "diff --git a/test.txt b/test.txt\n+Hello World".to_string();
  let result = create_commit_request(diff, 0, Model::GPT41Mini);

  assert!(result.is_ok(), "Should create request even with 0 tokens");
  let request = result.unwrap();
  assert_eq!(request.max_tokens, 0, "Should preserve 0 max_tokens");
}

#[test]
fn test_create_request_with_empty_diff() {
  // Corner case: empty diff
  let diff = "".to_string();
  let result = create_commit_request(diff.clone(), 1000, Model::GPT41Mini);

  assert!(result.is_ok(), "Should handle empty diff");
  let request = result.unwrap();
  assert_eq!(request.prompt, diff, "Should preserve empty diff");
  assert!(!request.system.is_empty(), "System prompt should not be empty");
}

#[test]
fn test_create_request_with_whitespace_only_diff() {
  // Corner case: whitespace-only diff
  let diff = "   \n\t\n   ".to_string();
  let result = create_commit_request(diff.clone(), 1000, Model::GPT41Mini);

  assert!(result.is_ok(), "Should handle whitespace-only diff");
  let request = result.unwrap();
  assert_eq!(request.prompt, diff, "Should preserve whitespace diff");
}

#[test]
fn test_create_request_preserves_model() {
  // Test that different models are preserved correctly
  let diff = "diff --git a/test.txt b/test.txt\n+Test".to_string();
  let models = vec![Model::GPT41Mini, Model::GPT45, Model::GPT41, Model::GPT41Nano];

  for model in models {
    let result = create_commit_request(diff.clone(), 1000, model);
    assert!(result.is_ok(), "Should work with model {:?}", model);

    let request = result.unwrap();
    assert_eq!(request.model, model, "Should preserve model type");
  }
}

#[test]
fn test_create_request_with_max_u16_tokens() {
  // Edge case: maximum token count
  let diff = "diff --git a/test.txt b/test.txt\n+Test".to_string();
  let max_tokens = usize::from(u16::MAX);

  let result = create_commit_request(diff, max_tokens, Model::GPT41Mini);
  assert!(result.is_ok(), "Should handle max u16 tokens");

  let request = result.unwrap();
  assert_eq!(request.max_tokens, u16::MAX, "Should cap at u16::MAX");
}

#[test]
fn test_create_request_with_overflow_tokens() {
  // Edge case: token count exceeding u16::MAX
  let diff = "diff --git a/test.txt b/test.txt\n+Test".to_string();
  let max_tokens = usize::from(u16::MAX) + 1000;

  let result = create_commit_request(diff, max_tokens, Model::GPT41Mini);
  assert!(result.is_ok(), "Should handle token overflow");

  let request = result.unwrap();
  assert_eq!(request.max_tokens, u16::MAX, "Should cap at u16::MAX on overflow");
}

// =============================================================================
// SECTION 2: Simple Test Cases
// =============================================================================

#[test]
fn test_create_request_with_simple_diff() {
  let diff = r#"diff --git a/test.txt b/test.txt
index 1234567..abcdefg 100644
--- a/test.txt
+++ b/test.txt
@@ -1,1 +1,2 @@
 Original line
+New line added
"#
  .to_string();

  let result = create_commit_request(diff.clone(), 1000, Model::GPT41Mini);
  assert!(result.is_ok(), "Should handle simple diff");

  let request = result.unwrap();
  assert_eq!(request.prompt, diff, "Diff should be preserved in prompt");
  assert!(!request.system.is_empty(), "System prompt should be generated");
  assert_eq!(request.max_tokens, 1000, "Max tokens should be preserved");
}

#[test]
fn test_create_request_with_file_addition() {
  let diff = r#"diff --git a/new_file.js b/new_file.js
new file mode 100644
index 0000000..1234567
--- /dev/null
+++ b/new_file.js
@@ -0,0 +1,5 @@
+function hello() {
+  console.log('Hello World');
+}
+
+export default hello;
"#
  .to_string();

  let result = create_commit_request(diff.clone(), 2000, Model::GPT45);
  assert!(result.is_ok(), "Should handle file addition");

  let request = result.unwrap();
  assert!(request.prompt.contains("new file mode"), "Should preserve new file marker");
  assert!(request.prompt.contains("/dev/null"), "Should preserve null source reference");
}

#[test]
fn test_create_request_with_file_deletion() {
  let diff = r#"diff --git a/old_file.js b/old_file.js
deleted file mode 100644
index 1234567..0000000
--- a/old_file.js
+++ /dev/null
@@ -1,3 +0,0 @@
-function deprecated() {
-  return 'old';
-}
"#
  .to_string();

  let result = create_commit_request(diff.clone(), 1500, Model::GPT41Mini);
  assert!(result.is_ok(), "Should handle file deletion");

  let request = result.unwrap();
  assert!(request.prompt.contains("deleted file mode"), "Should preserve deletion marker");
}

#[test]
fn test_create_request_with_file_rename() {
  let diff = r#"diff --git a/old_name.js b/new_name.js
similarity index 95%
rename from old_name.js
rename to new_name.js
index 1234567..abcdefg 100644
--- a/old_name.js
+++ b/new_name.js
@@ -1,3 +1,3 @@
-const OLD = 'value';
+const NEW = 'value';
"#
  .to_string();

  let result = create_commit_request(diff.clone(), 1000, Model::GPT41Mini);
  assert!(result.is_ok(), "Should handle file rename");

  let request = result.unwrap();
  assert!(request.prompt.contains("rename from"), "Should preserve rename metadata");
  assert!(request.prompt.contains("rename to"), "Should preserve rename metadata");
}

#[test]
fn test_token_counting_with_diff_content() {
  let model = Model::GPT41Mini;

  let small_diff = "diff --git a/a.txt b/a.txt\n+Hello";
  let medium_diff = r#"diff --git a/test.js b/test.js
index 1234..5678 100644
--- a/test.js
+++ b/test.js
@@ -1,10 +1,15 @@
 function example() {
-  console.log('old');
+  console.log('new');
+  return true;
 }
"#;

  let small_tokens = model.count_tokens(small_diff).unwrap();
  let medium_tokens = model.count_tokens(medium_diff).unwrap();

  assert!(small_tokens > 0, "Small diff should have tokens");
  assert!(medium_tokens > small_tokens, "Medium diff should have more tokens");
  assert!(medium_tokens < 1000, "Medium diff shouldn't be excessive");
}

// =============================================================================
// SECTION 3: Complex Test Cases
// =============================================================================

#[test]
fn test_create_request_with_multiple_files() {
  let diff = r#"diff --git a/file1.js b/file1.js
index 1111111..2222222 100644
--- a/file1.js
+++ b/file1.js
@@ -1,2 +1,3 @@
 const a = 1;
+const b = 2;

diff --git a/file2.js b/file2.js
index 3333333..4444444 100644
--- a/file2.js
+++ b/file2.js
@@ -1,2 +1,2 @@
-const old = true;
+const new = true;

diff --git a/file3.js b/file3.js
new file mode 100644
index 0000000..5555555
--- /dev/null
+++ b/file3.js
@@ -0,0 +1,3 @@
+function newFunc() {
+  return 'hello';
+}
"#
  .to_string();

  let result = create_commit_request(diff.clone(), 3000, Model::GPT45);
  assert!(result.is_ok(), "Should handle multiple file changes");

  let request = result.unwrap();
  assert!(request.prompt.contains("file1.js"), "Should contain first file");
  assert!(request.prompt.contains("file2.js"), "Should contain second file");
  assert!(request.prompt.contains("file3.js"), "Should contain third file");
}

#[test]
fn test_create_request_with_binary_file() {
  let diff = r#"diff --git a/image.png b/image.png
index 1234567..abcdefg 100644
Binary files a/image.png and b/image.png differ
"#
  .to_string();

  let result = create_commit_request(diff.clone(), 1000, Model::GPT41Mini);
  assert!(result.is_ok(), "Should handle binary file diff");

  let request = result.unwrap();
  assert!(request.prompt.contains("Binary files"), "Should preserve binary marker");
}

#[test]
fn test_create_request_with_special_characters() {
  let diff = r#"diff --git a/test.txt b/test.txt
index 1234567..abcdefg 100644
--- a/test.txt
+++ b/test.txt
@@ -1,3 +1,5 @@
 Regular text
+Special chars: @#$%^&*()_+-=[]{}|;':",./<>?
+Unicode: 你好世界 🚀 émojis
+Escaped: \n\t\r\\
"#
  .to_string();

  let result = create_commit_request(diff.clone(), 2000, Model::GPT41Mini);
  assert!(result.is_ok(), "Should handle special characters");

  let request = result.unwrap();
  assert!(request.prompt.contains("@#$%^&*"), "Should preserve special ASCII");
  assert!(request.prompt.contains("你好世界"), "Should preserve Unicode");
  assert!(request.prompt.contains("🚀"), "Should preserve emojis");
}

#[test]
fn test_create_request_with_large_diff() {
  // Create a large diff with many lines
  let mut diff = String::from("diff --git a/large.txt b/large.txt\nindex 1234..5678 100644\n--- a/large.txt\n+++ b/large.txt\n");

  // Add 1000 lines of changes
  for i in 0..1000 {
    diff.push_str(&format!("@@ -{},{} +{},{} @@\n", i * 10, 10, i * 10, 10));
    for j in 0..10 {
      diff.push_str(&format!("+New line {} {}\n", i, j));
    }
  }

  let result = create_commit_request(diff.clone(), 8000, Model::GPT45);
  assert!(result.is_ok(), "Should handle large diff");

  let request = result.unwrap();
  assert!(request.prompt.len() > 10000, "Large diff should be preserved");

  // Count tokens to ensure we can handle large inputs
  let model = Model::GPT45;
  let token_count = model.count_tokens(&diff).unwrap();
  assert!(token_count > 1000, "Large diff should have substantial token count");
}

#[test]
fn test_create_request_with_very_long_lines() {
  // Some diffs can have very long lines (minified code, data files, etc.)
  let long_line = "x".repeat(10000);
  let diff = format!(
    "diff --git a/data.txt b/data.txt\nindex 1234..5678 100644\n--- a/data.txt\n+++ b/data.txt\n@@ -1,1 +1,1 @@\n-old\n+{}\n",
    long_line
  );

  let result = create_commit_request(diff.clone(), 5000, Model::GPT45);
  assert!(result.is_ok(), "Should handle very long lines");

  let request = result.unwrap();
  assert!(request.prompt.contains(&long_line), "Should preserve long line");
}

#[test]
fn test_create_request_with_mixed_operations() {
  let diff = r#"diff --git a/added.txt b/added.txt
new file mode 100644
index 0000000..1111111
--- /dev/null
+++ b/added.txt
@@ -0,0 +1,1 @@
+New file content

diff --git a/modified.txt b/modified.txt
index 2222222..3333333 100644
--- a/modified.txt
+++ b/modified.txt
@@ -1,2 +1,2 @@
-Old content
+New content

diff --git a/deleted.txt b/deleted.txt
deleted file mode 100644
index 4444444..0000000
--- a/deleted.txt
+++ /dev/null
@@ -1,1 +0,0 @@
-Removed content

diff --git a/old.txt b/renamed.txt
similarity index 100%
rename from old.txt
rename to renamed.txt

diff --git a/image.png b/image.png
index 5555555..6666666 100644
Binary files a/image.png and b/image.png differ
"#
  .to_string();

  let result = create_commit_request(diff.clone(), 4000, Model::GPT45);
  assert!(result.is_ok(), "Should handle mixed operations");

  let request = result.unwrap();

  // Verify all operation types are preserved
  assert!(request.prompt.contains("new file mode"), "Should contain addition");
  assert!(request.prompt.contains("deleted file mode"), "Should contain deletion");
  assert!(request.prompt.contains("rename from"), "Should contain rename");
  assert!(request.prompt.contains("Binary files"), "Should contain binary");
  assert!(request.prompt.contains("modified.txt"), "Should contain modification");
}

#[test]
fn test_token_counting_consistency_with_complex_diff() {
  let model = Model::GPT41Mini;

  let complex_diff = r#"diff --git a/src/main.rs b/src/main.rs
index abc123..def456 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,10 +1,15 @@
 fn main() {
-    println!("Old version");
+    println!("New version");
+
+    // Added functionality
+    let x = compute_value();
+    println!("Result: {}", x);
 }

+fn compute_value() -> i32 {
+    42
+}
"#;

  // Count tokens multiple times to ensure consistency
  let counts: Vec<usize> = (0..5)
    .map(|_| model.count_tokens(complex_diff).unwrap())
    .collect();

  // All counts should be identical
  assert!(
    counts.windows(2).all(|w| w[0] == w[1]),
    "Token counting should be consistent, got: {:?}",
    counts
  );
}

#[test]
fn test_create_request_with_code_in_multiple_languages() {
  let diff = r#"diff --git a/main.rs b/main.rs
index 111..222 100644
--- a/main.rs
+++ b/main.rs
@@ -1,1 +1,2 @@
 fn main() {}
+fn new_func() {}

diff --git a/app.py b/app.py
index 333..444 100644
--- a/app.py
+++ b/app.py
@@ -1,1 +1,2 @@
 def hello():
+    print("world")

diff --git a/index.js b/index.js
index 555..666 100644
--- a/index.js
+++ b/index.js
@@ -1,1 +1,2 @@
 console.log('start');
+const x = 42;

diff --git a/main.go b/main.go
index 777..888 100644
--- a/main.go
+++ b/main.go
@@ -1,1 +1,3 @@
 func main() {
+	fmt.Println("hello")
 }
"#
  .to_string();

  let result = create_commit_request(diff.clone(), 5000, Model::GPT45);
  assert!(result.is_ok(), "Should handle multiple programming languages");

  let request = result.unwrap();
  assert!(request.prompt.contains("main.rs"), "Should contain Rust file");
  assert!(request.prompt.contains("app.py"), "Should contain Python file");
  assert!(request.prompt.contains("index.js"), "Should contain JavaScript file");
  assert!(request.prompt.contains("main.go"), "Should contain Go file");
}

#[test]
fn test_template_contains_required_sections() {
  let template = generate_instruction_template().unwrap();

  // Verify template has all required sections for the LLM
  let required_sections = vec![
    "Git Commit Message", "Core Requirements", "Algorithm", "Impact Score", "File-Level Division", "Commit Message Generation", "Examples",
  ];

  for section in required_sections {
    assert!(template.contains(section), "Template should contain section: {}", section);
  }
}

#[test]
fn test_request_structure_completeness() {
  let diff = "diff --git a/test.txt b/test.txt\n+test".to_string();
  let request = create_commit_request(diff.clone(), 1000, Model::GPT41Mini).unwrap();

  // Verify request has all required components
  assert!(!request.system.is_empty(), "System prompt should not be empty");
  assert_eq!(request.prompt, diff, "User prompt should match input diff");
  assert_eq!(request.max_tokens, 1000, "Max tokens should be set correctly");
  assert_eq!(request.model, Model::GPT41Mini, "Model should be set correctly");

  // Verify system prompt has reasonable length
  assert!(request.system.len() > 500, "System prompt should be substantial");
  assert!(request.system.len() < 50000, "System prompt shouldn't be excessive");
}

// =============================================================================
// SECTION 4: Integration Tests
// =============================================================================

#[test]
fn test_full_workflow_simple() {
  // End-to-end test: create a simple diff and ensure the full request pipeline works
  let simple_diff = r#"diff --git a/README.md b/README.md
index 123abc..456def 100644
--- a/README.md
+++ b/README.md
@@ -1,3 +1,4 @@
 # My Project

 This is a sample project.
+Added a new line of documentation.
"#
  .to_string();

  // Test the full workflow
  let model = Model::GPT41Mini;
  let template = generate_instruction_template().unwrap();
  let token_count = calculate_token_usage(&model).unwrap();
  let request = create_commit_request(simple_diff.clone(), 2000, model).unwrap();

  // Verify all components work together
  assert!(!template.is_empty(), "Template should be generated");
  assert!(token_count > 0, "Token count should be calculated");
  assert_eq!(request.system, template, "Request should use the template");
  assert_eq!(request.prompt, simple_diff, "Request should use the diff");
  assert_eq!(request.model, model, "Request should use the correct model");
}

#[test]
fn test_end_to_end_with_token_limits() {
  // Test that we can calculate tokens for both template and diff
  let model = Model::GPT45;
  let diff = r#"diff --git a/src/main.rs b/src/main.rs
index abc..def 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,10 @@
 fn main() {
     println!("Hello, world!");
+
+    // New functionality
+    let result = calculate();
+    println!("Result: {}", result);
 }
+
+fn calculate() -> i32 { 42 }
"#
  .to_string();

  // Calculate total tokens needed
  let template_tokens = calculate_token_usage(&model).unwrap();
  let diff_tokens = model.count_tokens(&diff).unwrap();
  let total_input_tokens = template_tokens + diff_tokens;

  // Ensure we can fit within model limits
  assert!(
    total_input_tokens < model.context_size(),
    "Total input tokens ({}) should fit within model context ({})",
    total_input_tokens,
    model.context_size()
  );

  // Create request with remaining tokens
  let remaining_tokens = model.context_size() - total_input_tokens;
  let request = create_commit_request(diff, remaining_tokens, model).unwrap();

  assert!(request.max_tokens > 0, "Should have tokens available for response");
}
