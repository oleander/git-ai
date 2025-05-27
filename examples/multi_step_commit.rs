use std::env;

use anyhow::Result;
use async_openai::Client;
use ai::multi_step_integration::{generate_commit_message_local, generate_commit_message_multi_step, parse_diff};

#[tokio::main]
async fn main() -> Result<()> {
  // Initialize logging
  env_logger::init();

  // Example git diff with multiple files
  let example_diff = r#"diff --git a/src/auth/jwt.rs b/src/auth/jwt.rs
new file mode 100644
index 0000000..1234567
--- /dev/null
+++ b/src/auth/jwt.rs
@@ -0,0 +1,89 @@
+use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
+use serde::{Deserialize, Serialize};
+use chrono::{Utc, Duration};
+use anyhow::Result;
+
+#[derive(Debug, Serialize, Deserialize)]
+pub struct Claims {
+    pub sub: String,
+    pub exp: usize,
+    pub iat: usize,
+}
+
+impl Claims {
+    pub fn new(user_id: String) -> Self {
+        let now = Utc::now();
+        let exp = (now + Duration::hours(24)).timestamp() as usize;
+        let iat = now.timestamp() as usize;
+
+        Self {
+            sub: user_id,
+            exp,
+            iat,
+        }
+    }
+}
+
+pub fn generate_token(user_id: String, secret: &str) -> Result<String> {
+    let claims = Claims::new(user_id);
+    let token = encode(
+        &Header::default(),
+        &claims,
+        &EncodingKey::from_secret(secret.as_ref())
+    )?;
+    Ok(token)
+}
+
+pub fn validate_token(token: &str, secret: &str) -> Result<Claims> {
+    let token_data = decode::<Claims>(
+        token,
+        &DecodingKey::from_secret(secret.as_ref()),
+        &Validation::default()
+    )?;
+    Ok(token_data.claims)
+}
diff --git a/src/middleware/auth.rs b/src/middleware/auth.rs
new file mode 100644
index 0000000..2345678
--- /dev/null
+++ b/src/middleware/auth.rs
@@ -0,0 +1,67 @@
+use actix_web::{
+    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
+    Error, HttpMessage,
+};
+use futures_util::future::LocalBoxFuture;
+use std::future::{ready, Ready};
+
+use crate::auth::jwt::validate_token;
+
+pub struct AuthMiddleware;
+
+impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
+where
+    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
+    S::Future: 'static,
+    B: 'static,
+{
+    type Response = ServiceResponse<B>;
+    type Error = Error;
+    type InitError = ();
+    type Transform = AuthMiddlewareService<S>;
+    type Future = Ready<Result<Self::Transform, Self::InitError>>;
+
+    fn new_transform(&self, service: S) -> Self::Future {
+        ready(Ok(AuthMiddlewareService { service }))
+    }
+}
+
+pub struct AuthMiddlewareService<S> {
+    service: S,
+}
+
+impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
+where
+    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
+    S::Future: 'static,
+    B: 'static,
+{
+    type Response = ServiceResponse<B>;
+    type Error = Error;
+    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;
+
+    forward_ready!(service);
+
+    fn call(&self, req: ServiceRequest) -> Self::Future {
+        // Extract token from Authorization header
+        let token = req
+            .headers()
+            .get("Authorization")
+            .and_then(|h| h.to_str().ok())
+            .and_then(|h| h.strip_prefix("Bearer "));
+
+        if let Some(token) = token {
+            // Validate token
+            match validate_token(token, "secret") {
+                Ok(claims) => {
+                    req.extensions_mut().insert(claims);
+                }
+                Err(_) => {
+                    return Box::pin(async { Err(actix_web::error::ErrorUnauthorized("Invalid token")) });
+                }
+            }
+        } else {
+            return Box::pin(async { Err(actix_web::error::ErrorUnauthorized("Missing token")) });
+        }
+
+        let fut = self.service.call(req);
+        Box::pin(async move { fut.await })
+    }
+}
diff --git a/package.json b/package.json
index 3456789..abcdefg 100644
--- a/package.json
+++ b/package.json
@@ -15,6 +15,8 @@
     "express": "^4.18.2",
     "mongoose": "^7.0.3",
     "dotenv": "^16.0.3",
+    "jsonwebtoken": "^9.0.0",
+    "bcrypt": "^5.1.0",
     "cors": "^2.8.5"
   },
   "devDependencies": {
diff --git a/tests/auth.test.js b/tests/auth.test.js
new file mode 100644
index 0000000..4567890
--- /dev/null
+++ b/tests/auth.test.js
@@ -0,0 +1,45 @@
+const { generateToken, validateToken } = require('../src/auth/jwt');
+const { authenticate } = require('../src/auth');
+
+describe('Authentication', () => {
+  describe('JWT Token', () => {
+    it('should generate a valid token', () => {
+      const userId = 'user123';
+      const token = generateToken(userId);
+
+      expect(token).toBeDefined();
+      expect(typeof token).toBe('string');
+    });
+
+    it('should validate a valid token', () => {
+      const userId = 'user123';
+      const token = generateToken(userId);
+      const decoded = validateToken(token);
+
+      expect(decoded.sub).toBe(userId);
+      expect(decoded.exp).toBeGreaterThan(Date.now() / 1000);
+    });
+
+    it('should reject an invalid token', () => {
+      expect(() => {
+        validateToken('invalid.token.here');
+      }).toThrow();
+    });
+  });
+
+  describe('User Authentication', () => {
+    it('should authenticate valid credentials', async () => {
+      const result = await authenticate('testuser', 'password123');
+      expect(result.success).toBe(true);
+      expect(result.token).toBeDefined();
+    });
+
+    it('should reject invalid credentials', async () => {
+      const result = await authenticate('testuser', 'wrongpassword');
+      expect(result.success).toBe(false);
+      expect(result.error).toBe('Invalid credentials');
+    });
+  });
+});
diff --git a/logo.png b/logo.png
index 1234567..abcdefg 100644
Binary files a/logo.png and b/logo.png differ
"#;

  println!("Multi-Step Git Commit Message Generation Example\n");
  println!("==============================================\n");

  // Check if we should use OpenAI or local generation
  if let Ok(_api_key) = env::var("OPENAI_API_KEY") {
    println!("Using OpenAI API for multi-step analysis...\n");

    let client = Client::new();
    let model = "gpt-4";

    match generate_commit_message_multi_step(&client, model, example_diff, Some(72)).await {
      Ok(message) => {
        println!("Generated commit message: {}\n", message);
      }
      Err(e) => {
        eprintln!("Error generating commit message: {}", e);
      }
    }
  } else {
    println!("No OPENAI_API_KEY found. Using local analysis...\n");

    match generate_commit_message_local(example_diff, Some(72)) {
      Ok(message) => {
        println!("Generated commit message: {}\n", message);
      }
      Err(e) => {
        eprintln!("Error generating commit message: {}", e);
      }
    }
  }

  // Demonstrate the step-by-step process
  println!("\nStep-by-Step Process:");
  println!("====================\n");

  // Parse and show files
  let files = parse_diff(example_diff)?;

  println!("1. Parsed {} files from diff:", files.len());
  for (i, file) in files.iter().enumerate() {
    println!("   File {}: {} ({})", i + 1, file.path, file.operation);
  }

  // Analyze each file
  println!("\n2. Analyzing each file:");
  use ai::multi_step_analysis::analyze_file;
  for file in &files {
    let analysis = analyze_file(&file.path, &file.diff_content, &file.operation);
    println!(
      "   {} -> +{} -{} lines, category: {}",
      file.path, analysis.lines_added, analysis.lines_removed, analysis.file_category
    );
  }

  // Calculate scores
  println!("\n3. Calculating impact scores:");
  use ai::multi_step_analysis::{calculate_impact_scores, FileDataForScoring};
  let files_data: Vec<FileDataForScoring> = files
    .iter()
    .map(|file| {
      let analysis = analyze_file(&file.path, &file.diff_content, &file.operation);
      FileDataForScoring {
        file_path:      file.path.clone(),
        operation_type: file.operation.clone(),
        lines_added:    analysis.lines_added,
        lines_removed:  analysis.lines_removed,
        file_category:  analysis.file_category,
        summary:        analysis.summary
      }
    })
    .collect();

  let score_result = calculate_impact_scores(files_data);
  for file in &score_result.files_with_scores {
    println!("   {} -> impact score: {:.2}", file.file_path, file.impact_score);
  }

  // Generate candidates
  println!("\n4. Generating commit message candidates:");
  use ai::multi_step_analysis::generate_commit_messages;
  let generate_result = generate_commit_messages(score_result.files_with_scores, 72);
  for (i, candidate) in generate_result.candidates.iter().enumerate() {
    println!("   Candidate {}: \"{}\"", i + 1, candidate);
  }

  println!("\n5. Reasoning: {}", generate_result.reasoning);

  Ok(())
}
