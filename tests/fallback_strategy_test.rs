use ai::generation::fallback::{generate_with_fallback, get_api_key, validate_api_key};
use ai::config::AppConfig;

#[tokio::test]
async fn test_fallback_strategy_no_api_key() {
  // Clear environment to ensure clean test
  std::env::remove_var("OPENAI_API_KEY");

  let test_diff = r#"diff --git a/src/auth.rs b/src/auth.rs
new file mode 100644
index 0000000..1234567
--- /dev/null
+++ b/src/auth.rs
@@ -0,0 +1,5 @@
+pub fn authenticate(user: &str, pass: &str) -> bool {
+    user == "admin" && pass == "secret"
+}
"#;

  let mut config = AppConfig::default();
  config.openai_api_key = None;

  let result = generate_with_fallback(test_diff, &config).await;

  // Should succeed with local generation
  assert!(result.is_ok(), "Expected fallback to succeed, got: {:?}", result.err());

  let message = result.unwrap();
  assert!(!message.is_empty(), "Generated message should not be empty");
  assert!(message.len() <= 72, "Message should respect default length limit"); // Default max_commit_length
}

#[tokio::test]
async fn test_fallback_strategy_invalid_api_key() {
  let test_diff = r#"diff --git a/src/test.py b/src/test.py
new file mode 100644
index 0000000..abc123
--- /dev/null
+++ b/src/test.py
@@ -0,0 +1,3 @@
+def hello():
+    return "Hello, World!"
"#;

  let mut config = AppConfig::default();
  config.openai_api_key = Some("<PLACE HOLDER FOR YOUR API KEY>".to_string());

  let result = generate_with_fallback(test_diff, &config).await;

  // Should succeed with local fallback
  assert!(result.is_ok(), "Expected fallback to succeed, got: {:?}", result.err());

  let message = result.unwrap();
  assert!(!message.is_empty(), "Generated message should not be empty");
}

#[test]
fn test_api_key_validation() {
  // Test various invalid keys
  assert!(validate_api_key(None).is_err());
  assert!(validate_api_key(Some("")).is_err());
  assert!(validate_api_key(Some("<PLACE HOLDER FOR YOUR API KEY>")).is_err());

  // Test valid key
  assert!(validate_api_key(Some("sk-valid-key-123")).is_ok());
}

#[test]
fn test_get_api_key_from_config() {
  // Test with valid config
  let mut config = AppConfig::default();
  config.openai_api_key = Some("test-key-123".to_string());
  assert!(get_api_key(&config).is_ok());

  // Test with invalid config
  config.openai_api_key = Some("<PLACE HOLDER FOR YOUR API KEY>".to_string());
  assert!(get_api_key(&config).is_err());

  // Test with no config
  config.openai_api_key = None;
  std::env::remove_var("OPENAI_API_KEY");
  assert!(get_api_key(&config).is_err());
}

#[test]
fn test_get_api_key_from_env() {
  // Set environment variable
  std::env::set_var("OPENAI_API_KEY", "env-test-key");

  let mut config = AppConfig::default();
  config.openai_api_key = None;

  let result = get_api_key(&config);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), "env-test-key");

  // Clean up
  std::env::remove_var("OPENAI_API_KEY");
}
