use ai::function_calling;

fn main() -> anyhow::Result<()> {
  // Test with default max length
  let tool = function_calling::create_commit_function_tool(None)?;
  let json = serde_json::to_string_pretty(&tool)?;
  println!("Function schema with default max length (72):");
  println!("{}", json);
  println!("\n---\n");

  // Test with custom max length
  let tool = function_calling::create_commit_function_tool(Some(100))?;
  let json = serde_json::to_string_pretty(&tool)?;
  println!("Function schema with custom max length (100):");
  println!("{}", json);

  Ok(())
}
