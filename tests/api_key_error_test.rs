use async_openai::Client;
use async_openai::config::OpenAIConfig;
use ai::multi_step_integration::generate_commit_message_multi_step;

#[tokio::test]
async fn test_invalid_api_key_propagates_error() {
    // Initialize logging to capture warnings
    let _ = env_logger::builder().is_test(true).try_init();

    // Create a client with an invalid API key that matches the issue
    let config = OpenAIConfig::new().with_api_key("dl://BA7invalid_key_here");
    let client = Client::with_config(config);

    let example_diff = r#"diff --git a/test.txt b/test.txt
new file mode 100644
index 0000000..0000000
--- /dev/null
+++ b/test.txt
@@ -0,0 +1 @@
+Hello World
"#;

    // This should fail with an API key error, not log a warning and continue
    let result = generate_commit_message_multi_step(&client, "gpt-4o-mini", example_diff, Some(72)).await;

    // Verify the behavior - it should return an error, not continue with other files
    assert!(result.is_err(), "Expected API key error to be propagated as error, not warning");
    
    let error_message = result.unwrap_err().to_string();
    println!("Actual error message: '{}'", error_message);
    
    // Now it should properly detect authentication failures
    assert!(
        error_message.contains("OpenAI API authentication failed") || 
        error_message.contains("API key"),
        "Expected error message to indicate authentication failure, got: {}",
        error_message
    );
}