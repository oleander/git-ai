use ai::function_calling;
use anyhow::Result;

fn main() -> Result<()> {
  // Create the commit function tool
  let tool = function_calling::create_commit_function_tool(Some(72))?;

  // Print the tool definition
  println!("Commit Function Tool Definition:");
  println!("{tool:#?}");

  // Example of parsing a function response
  let example_response = r#"{
        "reasoning": "The diff shows updates to the OpenAI integration to support function calling for commit message generation",
        "message": "Add function calling support for commit message generation",
        "files": {
            "src/function_calling.rs": {
                "type": "added",
                "summary": "New module for function calling types and commit function definition"
            },
            "src/openai.rs": {
                "type": "modified",
                "summary": "Updated to use function calling instead of plain text responses"
            },
            "resources/prompt.md": {
                "type": "modified",
                "summary": "Updated prompt to include function calling instructions and examples"
            }
        }
    }"#;

  let parsed = function_calling::parse_commit_function_response(example_response)?;

  println!("\nParsed Function Response:");
  println!("Message: {}", parsed.message);
  println!("Reasoning: {}", parsed.reasoning);
  println!("Files:");
  for (path, change) in &parsed.files {
    println!("  {} ({}): {}", path, change.change_type, change.summary);
  }

  Ok(())
}
