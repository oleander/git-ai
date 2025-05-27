use ai::multi_step_integration::generate_commit_message_local;

#[test]
fn test_multi_step_is_default() {
  // Simple test to ensure multi-step generation works
  let test_diff = r#"diff --git a/src/main.rs b/src/main.rs
index 1234567..abcdefg 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,5 @@
 fn main() {
-    println!("Hello");
+    println!("Hello, world!");
+    // Added more functionality
+    println!("This is a test");
 }
"#;

  // Test that local multi-step generation works
  let result = generate_commit_message_local(test_diff, Some(72));
  assert!(result.is_ok());

  let message = result.unwrap();
  assert!(!message.is_empty());
  assert!(message.len() <= 72);

  // The message should be something meaningful about updating main.rs
  println!("Generated message: {}", message);
}

#[tokio::test]
async fn test_multi_step_with_openai_fallback() {
  use ai::openai::generate_commit_message;

  let test_diff = r#"diff --git a/src/auth.rs b/src/auth.rs
new file mode 100644
index 0000000..1234567
--- /dev/null
+++ b/src/auth.rs
@@ -0,0 +1,10 @@
+pub fn authenticate(user: &str, pass: &str) -> bool {
+    // Simple authentication logic
+    user == "admin" && pass == "secret"
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    // Tests would go here
+}
"#;

  // This will use multi-step by default, falling back as needed
  let result = generate_commit_message(test_diff).await;

  // Should always succeed with some fallback
  assert!(result.is_ok());

  let message = result.unwrap();
  assert!(!message.is_empty());

  println!("Generated message via default flow: {}", message);
}
