use anyhow::Result;
use ai::multi_step_integration::{generate_commit_message_parallel, parse_diff};
use async_openai::Client;

/// Demonstrates the new parallel commit message generation approach
/// This example shows how the parallel algorithm processes multiple files concurrently
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to see the parallel processing in action
    env_logger::init();

    println!("Parallel Commit Message Generation Demo");
    println!("======================================");
    println!();

    // Example multi-file diff to demonstrate parallel processing
    let multi_file_diff = r#"diff --git a/src/auth.rs b/src/auth.rs
index 1234567..abcdefg 100644
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -1,8 +1,15 @@
+use crate::security::hash;
+use crate::database::UserStore;
+
 pub struct AuthService {
     users: HashMap<String, User>,
 }

 impl AuthService {
+    pub fn new(store: UserStore) -> Self {
+        Self { users: store.load_users() }
+    }
+
     pub fn authenticate(&self, username: &str, password: &str) -> Result<Token> {
-        // Simple hardcoded check
-        if username == "admin" && password == "secret" {
+        // Enhanced security with proper hashing
+        let hashed = hash(password);
+        if self.users.get(username).map(|u| &u.password_hash) == Some(&hashed) {
             Ok(Token::new(username))
         } else {
             Err(AuthError::InvalidCredentials)
diff --git a/src/main.rs b/src/main.rs
index abcd123..efgh456 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,8 +1,12 @@
+mod auth;
+mod security;
+mod database;
+
 use std::collections::HashMap;

 fn main() {
     println!("Starting application");
     
-    // TODO: Add authentication
+    let auth = auth::AuthService::new(database::UserStore::new());
+    println!("Authentication service initialized");
 }
diff --git a/Cargo.toml b/Cargo.toml
index 9876543..1111111 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -6,4 +6,6 @@ edition = "2021"
 [dependencies]
 serde = "1.0"
 tokio = "1.0"
+bcrypt = "0.14"
+sqlx = "0.7"
"#;

    println!("1. Parsing diff to identify files for parallel processing...");
    let parsed_files = parse_diff(multi_file_diff)?;
    println!("   Found {} files to analyze:", parsed_files.len());
    for (i, file) in parsed_files.iter().enumerate() {
        println!("   {}. {} ({})", i + 1, file.path, file.operation);
    }
    println!();

    println!("2. Demonstrating the parallel analysis approach:");
    println!("   - Each file will be analyzed concurrently (not sequentially)");
    println!("   - Uses simple text completion (not complex function calling)"); 
    println!("   - Single synthesis step replaces 3 sequential API calls");
    println!();

    // Note: This would require a valid OpenAI API key to actually run
    // For the demo, we just show the structure
    if std::env::var("OPENAI_API_KEY").is_ok() {
        println!("3. Running parallel analysis (requires OpenAI API key)...");
        
        let client = Client::new();
        let model = "gpt-4o-mini";
        
        match generate_commit_message_parallel(&client, model, multi_file_diff, Some(72)).await {
            Ok(message) => {
                println!("   ✓ Generated commit message: '{}'", message);
                println!("   ✓ Message length: {} characters", message.len());
            }
            Err(e) => {
                println!("   ⚠ API call failed (expected without valid key): {}", e);
            }
        }
    } else {
        println!("3. Skipping API call (no OPENAI_API_KEY found)");
        println!("   Set OPENAI_API_KEY environment variable to test with real API");
    }
    
    println!();
    println!("Performance Benefits:");
    println!("• Single file: ~6.6s → ~4s (eliminate 2 sequential round-trips)");
    println!("• Multiple files: Linear scaling vs sequential (5 files: ~4.3s vs ~16s)");
    println!("• Better error resilience: Continue if some files fail to analyze");
    println!();
    
    println!("Architecture Improvements:");
    println!("• Two-phase design: Parallel analysis → Unified synthesis");
    println!("• Simplified API: Plain text responses vs function calling schemas");
    println!("• Graceful fallback: Falls back to original multi-step if parallel fails");

    Ok(())
}