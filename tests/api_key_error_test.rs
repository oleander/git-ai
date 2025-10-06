use ai::commit;
use ai::config::AppConfig;
use ai::model::Model;

#[tokio::test]
async fn test_invalid_api_key_propagates_error() {
  // Initialize logging to capture warnings
  let _ = env_logger::builder().is_test(true).try_init();

  // Ensure no API key is available from environment to force early validation failure
  let original_key = std::env::var("OPENAI_API_KEY").ok();
  std::env::remove_var("OPENAI_API_KEY");

  // Create settings with an invalid API key that fails early validation (no network calls)
  let settings = AppConfig {
    openai_api_key: Some("".to_string()), // Empty string triggers early validation failure
    model: Some("gpt-4o-mini".to_string()),
    max_tokens: Some(1024),
    max_commit_length: Some(72),
    timeout: Some(30)
  };

  let example_diff = "diff --git a/test.txt b/test.txt\n+Hello World".to_string();

  // This should fail with an API key error, not log a warning and continue
  let result = commit::generate(example_diff, 1024, Model::GPT41Mini, Some(&settings)).await;

  // Restore original environment variable if it existed
  if let Some(key) = original_key {
    std::env::set_var("OPENAI_API_KEY", key);
  }

  // Verify the behavior - it should return an error, not continue with other files
  assert!(result.is_err(), "Expected API key error to be propagated as error, not warning");

  let error_message = result.unwrap_err().to_string();
  println!("Actual error message: '{}'", error_message);

  // The error should indicate that the API key is not configured (early validation without network calls)
  assert!(
    error_message.contains("OpenAI API key not configured") || error_message.contains("Invalid OpenAI API key"),
    "Expected error message to indicate API key configuration issue, got: {}",
    error_message
  );
}
