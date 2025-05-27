use ai::function_calling;
use async_openai::types::ChatCompletionToolType;
use serde_json::json;

#[test]
fn test_create_commit_function_tool_default() {
  let tool = function_calling::create_commit_function_tool(None).unwrap();

  assert_eq!(tool.r#type, ChatCompletionToolType::Function);
  assert_eq!(tool.function.name, "commit");
  assert_eq!(
    tool.function.description.as_deref(),
    Some("Generate a git commit message based on the provided diff")
  );

  // Check that the parameters include maxLength with default value
  let params = tool.function.parameters.as_ref().unwrap();
  let properties = params.get("properties").unwrap();
  let message_props = properties.get("message").unwrap();
  let max_length = message_props.get("maxLength").unwrap();
  assert_eq!(max_length, 72);
}

#[test]
fn test_create_commit_function_tool_custom_length() {
  let tool = function_calling::create_commit_function_tool(Some(100)).unwrap();

  // Check that the parameters include maxLength with custom value
  let params = tool.function.parameters.as_ref().unwrap();
  let properties = params.get("properties").unwrap();
  let message_props = properties.get("message").unwrap();
  let max_length = message_props.get("maxLength").unwrap();
  assert_eq!(max_length, 100);
}

#[test]
fn test_parse_commit_function_response() {
  let response_json = json!({
      "reasoning": "The diff shows a new feature for AI-powered commit message generation",
      "message": "Add AI-powered commit message generation",
      "files": {
          "src/ai.rs": {
              "type": "added",
              "summary": "New AI module for generating commit messages",
              "lines_changed": 120,
              "impact_score": 0.9,
              "file_category": "source"
          },
          "src/main.rs": {
              "type": "modified",
              "summary": "Integrated AI module into main application",
              "lines_changed": 15,
              "impact_score": 0.7,
              "file_category": "source"
          }
      }
  });

  let response_str = serde_json::to_string(&response_json).unwrap();
  let parsed = function_calling::parse_commit_function_response(&response_str).unwrap();

  assert_eq!(parsed.message, "Add AI-powered commit message generation");
  assert_eq!(parsed.reasoning, "The diff shows a new feature for AI-powered commit message generation");
  assert_eq!(parsed.files.len(), 2);

  let ai_file = parsed.files.get("src/ai.rs").unwrap();
  assert_eq!(ai_file.change_type, "added");
  assert_eq!(ai_file.summary, "New AI module for generating commit messages");

  let main_file = parsed.files.get("src/main.rs").unwrap();
  assert_eq!(main_file.change_type, "modified");
  assert_eq!(main_file.summary, "Integrated AI module into main application");
}

#[test]
fn test_parse_commit_function_response_invalid_json() {
  let invalid_json = "{ invalid json }";
  let result = function_calling::parse_commit_function_response(invalid_json);
  assert!(result.is_err());
}

#[test]
fn test_parse_commit_function_response_missing_fields() {
  let incomplete_json = json!({
      "message": "Test commit"
      // Missing reasoning and files
  });

  let response_str = serde_json::to_string(&incomplete_json).unwrap();
  let result = function_calling::parse_commit_function_response(&response_str);
  assert!(result.is_err());
}
