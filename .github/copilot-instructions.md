# GitHub Copilot Instructions for Git AI

## Project Overview

Git AI is a Rust-based tool that automates intelligent commit message generation using AI. It uses a sophisticated multi-step analysis process to analyze git diffs, score file impacts, and generate meaningful commit messages via OpenAI's API.

### Core Philosophy

- **Multi-step analysis over single-shot prompts**: Divide-and-conquer approach analyzing files individually
- **Intelligent fallbacks**: Graceful degradation from API → Local → Single-step
- **Performance-first**: Parallel processing, efficient token management, smart truncation
- **Developer experience**: Clear, actionable commit messages following conventional commit format

## Architecture Components

### Key Modules

- `src/main.rs` - CLI interface and configuration management
- `src/bin/hook.rs` - Git prepare-commit-msg hook entry point
- `src/hook.rs` - Core hook logic, diff processing, parallel optimization (725 lines)
- `src/multi_step_analysis.rs` - File analysis and impact scoring
- `src/multi_step_integration.rs` - Multi-step orchestration and candidate generation
- `src/function_calling.rs` - OpenAI function calling and structured outputs
- `src/commit.rs` - Commit message generation and formatting (282 lines)
- `src/openai.rs` - OpenAI API integration
- `src/ollama.rs` - Ollama support for local models
- `src/config.rs` - Configuration management (INI-based)
- `src/client.rs` - HTTP client and API communication

## Coding Conventions

### Rust Standards

- **Edition**: 2021
- **Minimum Rust version**: 1.70+
- **Style**: Follow `rustfmt.toml` formatting rules
- **Module structure**: Prefer nested module organization (see `structure` rule)

### Error Handling

- Use `anyhow::Result` for application-level errors with context
- Use `thiserror` for defining custom error types
- Always add context with `.context()` or `.with_context()` when propagating errors
- Example:

  ```rust
  use anyhow::{Context, Result};

  fn read_config() -> Result<Config> {
      let content = fs::read_to_string("config.ini")
          .context("Failed to read config file")?;
      parse_config(&content)
          .context("Failed to parse configuration")
  }
  ```

### Async Patterns

- Use `tokio` runtime with full features
- Prefer `async/await` over manual futures
- Use `futures` crate for stream operations and combinators
- All API calls should be async

### Parallelism

- Use `rayon` for CPU-bound parallel operations (file analysis)
- Use `tokio::spawn` for concurrent async tasks
- Example: Parallel file analysis in `multi_step_analysis.rs`

### Code Organization

- Keep functions focused and single-purpose
- Extract complex logic into helper functions
- Use descriptive variable names (avoid abbreviations)
- Group related functionality in modules

## Multi-Step Analysis Pattern

When working with the multi-step system, follow this flow:

1. **Parse** - Split diff into individual files with metadata
2. **Analyze** - Use OpenAI function calling to extract structured data per file
3. **Score** - Calculate impact scores using weighted formula
4. **Generate** - Create multiple commit message candidates (action/component/impact focused)
5. **Select** - Choose best message based on highest impact files

### Scoring Formula

```rust
// Operation weights
add: 0.3, modify: 0.2, delete: 0.25, rename: 0.1, binary: 0.05

// Category weights
source: 0.4, test: 0.2, config: 0.25, build: 0.3, docs: 0.1, binary: 0.05

// Total score = (operation_weight + category_weight + lines_normalized) capped at 1.0
```

## Testing Requirements

### Unit Tests

- Write tests for all public functions
- Use `#[cfg(test)]` modules in the same file
- Test both success and error cases
- Example location: `tests/model_token_test.rs`, `tests/patch_test.rs`

### Integration Tests

- Place in `tests/` directory
- Use `tests/common.rs` for shared test utilities
- Test end-to-end workflows
- Run via `./scripts/integration-tests`

### Test Naming

```rust
#[test]
fn test_parse_diff_with_multiple_files() { /* ... */ }

#[test]
fn test_score_calculation_for_high_impact_source_file() { /* ... */ }
```

### Real Test Examples from Codebase

```rust
// Function calling tests (tests/function_calling_test.rs)
#[test]
fn test_create_commit_function_tool_default() {
    let tool = create_commit_function_tool(None).unwrap();
    assert_eq!(tool.function.name, "commit");
    // Verify JSON schema structure
}

#[test]
fn test_parse_commit_function_response_invalid_json() {
    let response = CommitFunctionCall {
        name: "commit".to_string(),
        arguments: "invalid json".to_string(),
    };
    let result = parse_commit_function_response(response);
    assert!(result.is_err());
}

// Multi-step integration tests (src/multi_step_integration.rs)
#[test]
fn test_parse_diff_with_c_i_prefixes() {
    let diff = r#"diff --git a/src/config.rs b/src/config.rs
index abc123..def456 100644
--- a/src/config.rs
+++ b/src/config.rs
@@ -1,3 +1,4 @@
+use anyhow::Result;
 use serde::Deserialize;
"#;
    let files = parse_diff(diff).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "src/config.rs");
    assert_eq!(files[0].operation, "modified");
}

// Model token tests (tests/model_token_test.rs)
#[test]
fn test_token_counting_accuracy() {
    let model = Model::Gpt4;
    let text = "Hello world";
    let tokens = model.count_tokens(text).unwrap();
    assert!(tokens > 0);
    assert!(tokens < 10); // Reasonable bounds
}

// Hook tests (src/hook.rs)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_pool_get() {
        let mut pool = StringPool::new();
        let result = pool.get("test");
        assert_eq!(result, "test");
    }

    #[test]
    fn test_string_pool_limit() {
        let mut pool = StringPool::new();
        // Test memory limits
        for i in 0..1000 {
            pool.put(format!("test_{}", i));
        }
        assert!(pool.len() <= StringPool::MAX_SIZE);
    }
}
```

### Integration Test Patterns

```rust
// Use tests/common.rs for shared utilities
use crate::common::*;

// Test helper functions pattern
fn setup_test_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    // Initialize git repo, add files, etc.
    temp_dir
}

fn create_test_diff(file_content: &str) -> String {
    format!(r#"diff --git a/test.rs b/test.rs
index abc123..def456 100644
--- a/test.rs
+++ b/test.rs
@@ -1,1 +1,1 @@
-// Old content
+{}"#, file_content)
}

#[tokio::test]
async fn test_end_to_end_commit_generation() {
    let _temp_repo = setup_test_repo();
    let diff = create_test_diff("// New content");
    
    let result = generate_commit_message_local(diff, Some(72));
    assert!(result.is_ok());
    
    let message = result.unwrap();
    assert!(message.len() <= 72);
    assert!(!message.is_empty());
}
```

## Verification Workflow

### ALWAYS Verify Changes Before Committing

**Critical**: Run the complete test suite before AND after making any changes to ensure nothing breaks.

#### Step 1: Baseline Verification (Before Changes)

```bash
# Run all unit and integration tests to establish baseline
cargo test --all

# Run comprehensive test suite
./scripts/comprehensive-tests

# Verify specific integration scenarios
./scripts/integration-tests

# Test hook functionality if modifying hook-related code
./scripts/hook-stress-test

# Check for compilation warnings
cargo clippy -- -D warnings

# Verify formatting
cargo fmt -- --check
```

#### Step 2: Make Your Changes

- Implement your feature or fix
- Add new tests for new functionality
- Update existing tests if behavior changes
- Add documentation for public APIs

#### Step 3: Post-Change Verification

```bash
# Run the same test suite again
cargo test --all

# Verify integration tests still pass
./scripts/integration-tests

# Check for new linter warnings
cargo clippy -- -D warnings

# Ensure code is properly formatted
cargo fmt

# Build in release mode to catch optimization issues
cargo build --release

# Test locally installed version
just local-install
# Then test in a real git repository
```

#### Step 4: Verify Specific Scenarios

**If you modified hook logic** (`src/hook.rs`, `src/bin/hook.rs`):

```bash
./scripts/hook-stress-test
# Test with actual git commits in a test repository
cd test-repo
git add .
git commit --no-edit
```

**If you modified multi-step analysis** (`src/multi_step_*.rs`):

```bash
# Run specific multi-step tests
cargo test multi_step
# Check examples
cargo run --example multi_step_commit
```

**If you modified OpenAI integration** (`src/openai.rs`, `src/function_calling.rs`):

```bash
# Run function calling tests
cargo test function_calling
# Test with real API (requires OPENAI_API_KEY)
cargo run --example function_calling_demo
```

**If you modified commit message generation** (`src/commit.rs`):

```bash
# Run commit-related tests
cargo test commit
# Verify output format
cargo test --test llm_input_generation_test
```

### Continuous Verification During Development

```bash
# Watch mode for rapid feedback during development
cargo watch -x test -x clippy

# Run specific test while developing
cargo test test_name -- --nocapture

# Check test coverage (if using tarpaulin)
cargo tarpaulin --out Html
```

### What to Check For

✅ **All tests pass** - No test failures or panics
✅ **No new clippy warnings** - Code quality maintained
✅ **Proper formatting** - Follows rustfmt.toml rules
✅ **No performance regressions** - Tests complete in reasonable time
✅ **Integration tests work** - End-to-end scenarios function correctly
✅ **Hook functionality intact** - Can generate commit messages in real repos
✅ **Documentation updated** - README and code comments reflect changes
✅ **Error messages clear** - New errors provide actionable information

### Red Flags to Watch For

🚫 **Test failures** - Must be fixed before proceeding
🚫 **Clippy warnings** - Address or explicitly allow with justification
🚫 **Compilation errors** - Obviously must be resolved
🚫 **Hanging tests** - Indicates deadlock or infinite loop
🚫 **Flaky tests** - Tests that pass/fail inconsistently need investigation
🚫 **Silent failures** - Code that compiles but doesn't work as expected

### When Suggesting Code Changes

Always include verification steps in your suggestions:

```markdown
1. Make the following changes to [file]
2. Run `cargo test` to verify tests still pass
3. Run `cargo clippy` to check for warnings
4. Test the functionality with: [specific test command]
5. Verify in a real git repository if hook-related
```

## Performance Considerations

### Token Management

- Default max tokens: 512
- Track token usage with `tiktoken-rs`
- Truncate diffs intelligently (keep file boundaries)
- Prioritize high-impact files in limited contexts

### Optimization Strategies

- Parallel file analysis with `rayon`
- Lazy evaluation where possible
- Cache API responses when appropriate
- Use `parking_lot` for low-overhead locking

## Configuration Management

### Settings Format

- Use INI format via `config` crate
- Store in `~/.config/git-ai/config.ini`
- Support environment variables via `dotenv`

### Required Settings

- `openai-api-key` - OpenAI API key (required)
- `model` - AI model (default: `gpt-4.1`)
- `max-tokens` - Token limit (default: 512)
- `max-commit-length` - Character limit (default: 72)

## Git Integration

### Hook Installation

- Symlink `git-ai-hook` binary to `.git/hooks/prepare-commit-msg`
- Detect existing hooks and warn user
- Support install/uninstall/reinstall commands

### Diff Processing

- Handle multiple diff formats (standard, commit with hash, raw)
- Support all operation types: add, modify, delete, rename, binary
- Extract file paths, content, and metadata
- Categorize files: source, test, config, docs, build, binary

## OpenAI Integration

### Function Calling

- Use structured outputs for file analysis
- Define JSON schemas for commit message generation
- Handle API errors gracefully with retries
- Support multiple models: gpt-4.1, gpt-4o, gpt-4o-mini, gpt-4

### Assistant API

- Maintain per-project threads for context
- Host exclusive assistant instance locally
- Accumulate learning across all projects

### AI Provider Integration

#### OpenAI Integration (`src/openai.rs`)

```rust
use async_openai::Client;
use async_openai::types::{
    CreateChatCompletionRequest, 
    ChatCompletionRequestMessage,
    ChatCompletionTool
};

pub async fn call_with_config(
    client: &Client<async_openai::config::OpenAIConfig>,
    model: &Model,
    messages: Vec<ChatCompletionRequestMessage>,
    tools: Option<Vec<ChatCompletionTool>>,
    max_tokens: Option<u16>
) -> Result<String> {
    let request = CreateChatCompletionRequest {
        model: model.to_string(),
        messages,
        tools,
        max_tokens: max_tokens.map(|t| t as u32),
        temperature: Some(0.1), // Low for consistency
        ..Default::default()
    };

    let response = client.chat().create(request).await?;
    // Handle structured outputs and function calls
    parse_openai_response(response)
}
```

#### Ollama Integration (`src/ollama.rs`)

```rust
// Local model support for privacy and offline use
pub struct OllamaClient {
    base_url: String,
    model: String,
}

impl OllamaClient {
    pub async fn generate(&self, prompt: &str) -> Result<String> {
        // HTTP client for Ollama REST API
        let client = reqwest::Client::new();
        let response = client
            .post(&format!("{}/api/generate", self.base_url))
            .json(&json!({
                "model": self.model,
                "prompt": prompt,
                "stream": false
            }))
            .send()
            .await?;
        
        let result: OllamaResponse = response.json().await?;
        Ok(result.response)
    }
}
```

#### Model-Specific Optimizations

```rust
impl Model {
    pub fn count_tokens(&self, text: &str) -> Result<usize> {
        match self {
            Model::Gpt4 | Model::Gpt4o | Model::Gpt4oMini | Model::Gpt41 => {
                // Use tiktoken for OpenAI models
                tiktoken_count_tokens(text, self.encoding())
            }
            Model::Ollama(_) => {
                // Rough estimation for local models
                estimate_tokens_by_chars(text)
            }
        }
    }

    pub fn max_context_length(&self) -> usize {
        match self {
            Model::Gpt4 => 8192,
            Model::Gpt4o => 128000,
            Model::Gpt4oMini => 128000,
            Model::Gpt41 => 32768,
            Model::Ollama(_) => 4096, // Conservative default
        }
    }

    pub fn recommended_max_tokens(&self) -> u16 {
        match self {
            Model::Gpt4oMini => 256,  // Faster responses
            Model::Gpt4o => 1024,     // Higher quality
            _ => 512,                 // Balanced default
        }
    }
}
```

## Commit Message Format

### Style Guidelines

- Action-focused: "Add authentication to user service"
- Component-focused: "auth: implement JWT validation"
- Impact-focused: "New feature enabling secure login"
- Follow conventional commits when appropriate: `type(scope): description`
- Respect max length configuration (default 72 chars)
- Be descriptive but concise
- Focus on **what** and **why**, not **how**

## CLI Interface

### Command Structure

```bash
# Configuration management
git-ai config set <key> <value>  # Set configuration
git-ai config get <key>          # Get configuration value
git-ai config reset              # Reset to defaults

# Hook management
git-ai hook install              # Install git hook
git-ai hook uninstall            # Remove git hook
git-ai hook reinstall            # Reinstall hook

# Available for debugging and testing
git-ai --help                    # Show help information
```

### Configuration Keys Reference

| Key | Default | Description | Example |
|-----|---------|-------------|---------|
| `openai-api-key` | (required) | OpenAI API key | `sk-...` |
| `model` | `gpt-4.1` | AI model to use | `gpt-4o`, `gpt-4o-mini`, `gpt-4` |
| `max-tokens` | `512` | API request token limit | `256`, `1024` |
| `max-commit-length` | `72` | Commit message character limit | `50`, `100` |

## Development Workflow

### Local Development

- Use `just local-install` for quick dev installation with hook setup
- Run `cargo test --all` before committing
- Check `./scripts/integration-tests` for comprehensive testing
- Run `./scripts/hook-stress-test` for hook testing
- Use `./scripts/comprehensive-tests` for full test coverage

### Justfile Commands

The project includes essential development commands:

```bash
just local-install        # Install locally with debug symbols and setup hooks
just integration-test      # Run integration tests in Docker
just docker-build         # Build Docker image
just local-github-actions # Run GitHub Actions locally with act
```

### Script Utilities

```bash
# Testing scripts (Fish shell)
./scripts/integration-tests      # Core integration test suite
./scripts/comprehensive-tests    # Extensive test coverage (11 test categories)
./scripts/hook-stress-test      # Git hook functionality testing
./scripts/current-version       # Get current version information
```

### Building

- **Development builds**: `cargo build` (fast compilation)
- **Release builds**: `cargo build --release` (LTO and aggressive optimization)
- **Debug symbols**: Include debug symbols even in release builds
- **Multi-target support**: Linux (GNU/musl), macOS, Windows
- **Local installation**: `cargo install --debug --path .` for development

## Documentation Standards

### Code Comments

- Document complex algorithms and business logic
- Use `///` for public API documentation
- Include examples in doc comments for public functions
- Keep comments up-to-date with code changes

### README Updates

- Update examples when adding features
- Document new configuration options in tables
- Update architecture diagrams if structure changes
- Add FAQ entries for common questions

## Dependencies

### Core Dependencies

- `anyhow` - Error handling with context
- `tokio` - Async runtime
- `serde`/`serde_json` - Serialization
- `async-openai` - OpenAI API client
- `git2` - Git operations
- `rayon` - Parallel processing
- `tiktoken-rs` - Token counting

### Minimal Features

- Use `default-features = false` when possible
- Only enable required features explicitly
- Keep binary size reasonable

## Security Considerations

### API Keys

- Never commit API keys to repository
- Store in config file or environment variables
- Use dotenv for development
- Clear error messages without exposing secrets

### Input Validation

- Validate all user inputs
- Sanitize file paths
- Check diff content for unusual patterns
- Handle binary files safely (don't process content)

## Best Practices When Generating Code

1. **Run tests BEFORE making changes**: Establish baseline with `cargo test --all`
2. **Run tests AFTER making changes**: Verify nothing broke with the same commands
3. **Prefer existing patterns**: Match the style in surrounding code
4. **Add context**: Use `.context()` on all `Result` returns
5. **Parallelize when beneficial**: Use `rayon` for independent operations
6. **Test thoroughly**: Include both unit and integration tests
7. **Document public APIs**: Use doc comments with examples
8. **Handle errors gracefully**: Provide clear, actionable error messages
9. **Optimize token usage**: Be mindful of API costs
10. **Follow conventional commits**: When generating commit messages
11. **Use structured logging**: Use `log` or `tracing` macros, not `println!`
12. **Keep functions focused**: Extract helpers for complex logic
13. **Verify with clippy**: Run `cargo clippy -- -D warnings` to catch issues
14. **Format consistently**: Run `cargo fmt` before committing

## Common Patterns

### Structured Function Calling

```rust
use serde::{Deserialize, Serialize};
use serde_json::json;
use async_openai::types::{ChatCompletionTool, ChatCompletionToolType, FunctionObjectArgs};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysisResult {
    pub lines_added: u32,
    pub lines_removed: u32,
    pub file_category: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    #[serde(rename = "type")]
    pub change_type: String,
    pub summary: String,
    pub lines_changed: u32,
    pub impact_score: f32,
    pub file_category: String,
}

// Create OpenAI function tools with JSON schemas
pub fn create_analyze_function_tool() -> Result<ChatCompletionTool> {
    let function = FunctionObjectArgs::default()
        .name("analyze")
        .description("Analyze a single file's changes from the git diff")
        .parameters(json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Relative path to the file"
                },
                "diff_content": {
                    "type": "string", 
                    "description": "The git diff content for this specific file only"
                },
                "operation_type": {
                    "type": "string",
                    "enum": ["added", "modified", "deleted", "renamed", "binary"],
                    "description": "Type of operation performed on the file"
                }
            },
            "required": ["file_path", "diff_content", "operation_type"]
        }))
        .build()?;

    Ok(ChatCompletionTool { 
        r#type: ChatCompletionToolType::Function, 
        function 
    })
}
```

### Fallback Strategy

```rust
// Try multi-step with API
if let Ok(msg) = multi_step_with_api(diff).await {
    return Ok(msg);
}

// Fallback to local analysis
if let Ok(msg) = local_multi_step(diff) {
    return Ok(msg);
}

// Last resort: single-step API
single_step_api(diff).await
```

### Parallel File Processing

```rust
use rayon::prelude::*;
use tokio::task::JoinSet;

// CPU-bound parallel processing with rayon
let analyses: Vec<FileAnalysis> = files
    .par_iter()
    .map(|file| analyze_file(file))
    .collect();

// Async parallel processing with tokio
let mut join_set = JoinSet::new();
for file in files {
    join_set.spawn(async move {
        analyze_file_async(&file).await
    });
}

let results = join_all(futures).await;
```

## Debugging and Troubleshooting

### Environment Setup for Debugging

```bash
# Enable debug logging for all components
export RUST_LOG=debug

# Enable trace logging for specific modules
export RUST_LOG=ai::multi_step_analysis=trace,ai::openai=debug

# Run with debug output
cargo run --example multi_step_commit
```

### Debug Output Analysis

Git AI provides comprehensive debug output when `RUST_LOG=debug` is set:

```
=== GIT AI HOOK DEBUG SESSION ===

📋 INITIALIZATION
  Args:        commit_msg_file='.git/COMMIT_EDITMSG', source=None, sha1=None
  Build:       Debug build with performance profiling enabled

⚙️  SETUP & PREPARATION  
  │ Generate instruction template     1.56ms    ✓
  │ Count tokens                      306.13ms  ✓
  │ Calculate instruction tokens      307.77ms  ✓
  └ Get context size                  959.00ns  ✓

🤖 AI PROCESSING
  Multi-Step Attempt:                           FAILED
    └ Error: Invalid function_call             ✗ No function named 'required' specified

  Single-Step Fallback:                        SUCCESS
    │ Creating commit function tool             ✓ max_length=72
    │ OpenAI API call                   2.78s   ✓
    └ Response parsing                          ✓
```

### Common Issues and Solutions

#### API-Related Issues

```bash
# Test API connectivity
cargo run --example function_calling_demo

# Check API key configuration
git-ai config get openai-api-key

# Test with different models
git-ai config set model gpt-4o-mini  # Faster, cheaper
git-ai config set model gpt-4o       # Better quality
```

#### Hook Issues

```bash
# Reinstall hook if not working
git-ai hook reinstall

# Check hook file directly
ls -la .git/hooks/prepare-commit-msg
cat .git/hooks/prepare-commit-msg

# Test in isolation
RUST_LOG=debug git commit --no-edit
```

#### Performance Issues

```bash
# Profile token usage
RUST_LOG=debug git-ai 2>&1 | grep "tokens"

# Monitor API call timing
RUST_LOG=debug git-ai 2>&1 | grep "OpenAI API call"

# Reduce token limit for large diffs
git-ai config set max-tokens 256
```

### Testing Specific Scenarios

```bash
# Test empty diffs
git commit --allow-empty --no-edit

# Test large diffs
dd if=/dev/zero of=large_file bs=1M count=1
git add large_file && git commit --no-edit

# Test binary files
cp /bin/ls binary_file && git add binary_file && git commit --no-edit

# Test Unicode and special characters
echo "🚀 Unicode test 你好" > unicode.txt && git add . && git commit --no-edit
```

### Integration Test Categories

The comprehensive test suite covers 11 categories:

1. **Hook Installation and Configuration** - Basic setup and config management
2. **Basic Git Operations** - Standard commit workflows
3. **File Creation Permutations** - Empty files, multiple files, different content types
4. **File Modification Permutations** - Start, middle, end modifications
5. **Advanced Git Operations** - Amend, squash, template commits
6. **Branch and Merge Operations** - Feature branches, merging
7. **File Operations** - Deletions, mixed operations
8. **Special Content** - Binary files, Unicode, special characters
9. **File System Operations** - Directory moves, symlinks, permissions
10. **Edge Cases** - Empty commits, case sensitivity, file/directory conversion
11. **Bulk Operations** - Many files, large changes

### Performance Monitoring

```rust
// Use profiling macros for performance tracking
use crate::profile;

pub fn expensive_operation() -> Result<String> {
    profile!("Generate instruction template");
    // ... implementation
}
```

## API Usage Optimization

### Token Management Strategies

```rust
// Smart truncation preserving file boundaries
let truncated_diff = if estimated_tokens > max_tokens {
    truncate_diff_intelligently(&diff, max_tokens)
} else {
    diff
};

// Prioritize high-impact files
let sorted_files: Vec<_> = files
    .iter()
    .sorted_by(|a, b| b.impact_score.partial_cmp(&a.impact_score).unwrap())
    .take(max_files_for_context)
    .collect();
```

### Model Selection Guidelines

| Model | Use Case | Speed | Quality | Cost |
|-------|----------|--------|---------|------|
| `gpt-4.1` | Default, balanced performance | Medium | High | Medium |
| `gpt-4o` | Best quality, complex diffs | Slow | Highest | High |
| `gpt-4o-mini` | Fast iteration, simple diffs | Fast | Good | Low |
| `gpt-4` | Stable, proven performance | Medium | High | High |

### Fallback Strategy Implementation

```rust
// Implement graceful degradation
pub async fn generate_commit_message(diff: String) -> Result<String> {
    // Try multi-step with API
    if let Ok(msg) = generate_commit_message_multi_step(&client, diff.clone()).await {
        return Ok(msg);
    }

    // Fallback to local analysis
    if let Ok(msg) = generate_commit_message_local(diff.clone(), Some(72)) {
        return Ok(msg);
    }

    // Last resort: single-step API
    generate_commit_message_single_step(&client, diff).await
}
```

## Release and Deployment

### Version Management

```bash
# Check current version
./scripts/current-version

# Prepare release build
cargo build --release

# Multi-platform builds
cargo build --target x86_64-unknown-linux-gnu
cargo build --target x86_64-unknown-linux-musl
cargo build --target x86_64-apple-darwin
```

### CI/CD Pipeline

```bash
# Test locally with act (GitHub Actions simulation)
just local-github-actions

# Run full release pipeline in Docker
just release
```

### Distribution Channels

- **Crates.io**: Primary distribution via `cargo install git-ai`
- **Pre-compiled binaries**: Via `cargo-binstall` for faster installation
- **Docker**: Containerized builds and testing
- **GitHub Releases**: Tagged releases with pre-built binaries

## Workspace Rules

The project includes two important workspace rules:

1. **cleanup**: Full repository cleanup for Rust projects (remove dead code, unused dependencies, outdated docs)
2. **structure**: Enforce nested module file structure (split underscored filenames into directory-based modules)

Follow these rules when refactoring or adding new code.

---

## Code Quality Standards

> **Cross-referenced with actual codebase** - All rules verified against Git AI v1.0.9

### Formatting (rustfmt.toml - Mandatory)

**Source**: `rustfmt.toml` - Enforced by CI

```toml
max_width = 140
tab_spaces = 2
hard_tabs = false
edition = "2021"
style_edition = "2021"

imports_granularity = "Module"
imports_layout = "Horizontal"
group_imports = "StdExternalCrate"
imports_indent = "Block"
reorder_imports = false
reorder_modules = false

fn_params_layout = "Compressed"
fn_call_width = 90
attr_fn_like_width = 120
reorder_impl_items = true

struct_lit_single_line = true
struct_lit_width = 50
struct_field_align_threshold = 40
use_field_init_shorthand = true

enum_discrim_align_threshold = 40
match_block_trailing_comma = false
match_arm_blocks = false

overflow_delimited_expr = true
use_small_heuristics = "Max"
force_multiline_blocks = true
chain_width = 60
trailing_comma = "Never"
```

**Verify**: `cargo fmt -- --check`

### Naming Conventions

**Types** - Descriptive, clear names (multi-word acceptable):

```rust
// ✅ Clear and descriptive (from actual codebase)
struct FileAnalysisResult { /* ... */ }
struct CommitFunctionArgs { /* ... */ }
struct ParsedFile { /* ... */ }
enum HookError { /* ... */ }

// ⚠️ Refactor candidates (overly verbose)
struct FileDataForScoring { /* ... */ }  // → FileData
struct FileWithScore { /* ... */ }       // → ScoredFile
```

**Functions** - Verb phrases, context-appropriate length:

```rust
// ✅ Descriptive verbs (from codebase)
fn parse_diff(content: &str) -> Result<Vec<ParsedFile>>
fn generate_commit_message(diff: &str) -> Result<String>
fn create_analyze_function_tool() -> Result<ChatCompletionTool>

// ❌ Generic without context
fn process() -> Result<()>
fn handle() -> Result<()>
```

**Constants** - SCREAMING_SNAKE_CASE, descriptive:

```rust
// ✅ From actual codebase
const MAX_POOL_SIZE: usize = 1000;
const DEFAULT_STRING_CAPACITY: usize = 8192;
const PARALLEL_CHUNK_SIZE: usize = 25;
```

**Modules** - Scope types, enable short names within:

```rust
mod diff {
    pub struct Parser { /* ... */ }
    pub fn parse() -> Result<ParsedFile>  // Short in context
}

mod generation {
    pub struct Strategy { /* ... */ }
    pub fn generate() -> Result<String>
}
```

### Type System

**Newtypes for domain modeling**:

```rust
struct ImpactScore(f32);
struct TokenCount(usize);
struct UserId(i64);
```

**Exhaustive enums over booleans**:

```rust
// ✅ From codebase
enum OperationType { Added, Modified, Deleted, Renamed, Binary }

// ❌ Anti-pattern
struct File { is_added: bool, is_modified: bool }
```

**Typestate pattern**:

```rust
struct Validated;
struct Email<S = Unvalidated> {
    value: String,
    _state: PhantomData<S>
}
```

**Standard derives** (from codebase):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Error, Debug)]
#[derive(Default)]  // When sensible default exists
#[derive(PartialEq, Eq)]  // Add Eq if PartialEq
```

### Error Handling

**thiserror for libraries**:

```rust
// ✅ From codebase (hook.rs)
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HookError {
    #[error("failed to open repository")]
    OpenRepository,
    #[error("failed to get patch")]
    GetPatch,
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error)
}
```

**anyhow for applications**:

```rust
// ✅ Pattern from codebase
use anyhow::{Context, Result, bail};

pub fn generate(diff: &str) -> Result<String> {
    let parsed = parse_diff(diff)
        .context("failed to parse git diff")?;

    if parsed.is_empty() {
        bail!("empty diff provided");
    }
    Ok(result)
}
```

**Eliminate unwrap/expect**:

**Guidance**: Minimize or eliminate the use of `unwrap` and `expect` in production code. Prefer error handling patterns that propagate errors with context.

```rust
// ❌ Avoid in production code
let value = option.unwrap();
let value = result.expect("this should never fail");

// ✅ Use ? operator with context
let value = option.ok_or_else(|| anyhow!("no value found"))?;
let value = result.context("operation failed")?;
```

### API Design

**Accept borrowed, return owned**:

```rust
// ✅ From codebase pattern
fn process(input: &str) -> String
fn parse(diff: &str) -> Result<Vec<ParsedFile>>
```

**Trait bounds for flexibility**:

```rust
fn process(path: impl AsRef<Path>) -> Result<()>
fn with_config(config: impl Into<Config>) -> Self
```

**Must-use for important returns**:

```rust
#[must_use = "commit message should be used or logged"]
pub fn generate_commit_message(diff: &str) -> Result<String>
```

### Performance

**Pre-allocate** (pattern from codebase):

```rust
// ✅ Always pre-allocate when size is known
let mut files = HashMap::with_capacity(ESTIMATED_FILES_COUNT);
let mut results = Vec::with_capacity(total_files);
let mut buffer = String::with_capacity(DEFAULT_STRING_CAPACITY);
```

**Iterators over loops**:

```rust
// ✅ Iterator chains
let scored: Vec<_> = files
    .into_iter()
    .filter(|f| f.lines_changed > 0)
    .map(calculate_score)
    .collect();
```

**Inline hot paths**:

```rust
// ✅ Small, frequently-called functions
#[inline]
pub fn calculate_score(data: &FileData) -> f32 { /* ... */ }

// ❌ Don't inline large functions
#[inline]  // Bloats binary
pub async fn generate_commit_message(diff: &str) -> Result<String> {
    // 100+ lines
}
```

**Parallel processing** (Rayon - from codebase):

```rust
use rayon::prelude::*;

// ✅ Parallel iterator for CPU-bound work
let results: Vec<_> = files
    .par_iter()
    .map(process_file)
    .collect();

// ✅ Chunking for better cache locality
let chunks: Vec<_> = files.chunks(PARALLEL_CHUNK_SIZE).collect();
chunks.par_iter().try_for_each(process_chunk)?;
```

### Async (Tokio 1.45.1)

**Structured concurrency** (from multi_step_integration.rs):

```rust
use futures::future::join_all;

// ✅ join! over spawn
let futures: Vec<_> = files
    .iter()
    .map(|f| analyze_file(client, f))
    .collect();

let results = join_all(futures).await;

// ❌ Unstructured spawning
for file in files {
    tokio::spawn(analyze_file(client, file));
}
```

**spawn_blocking for CPU-bound**:

```rust
let result = tokio::task::spawn_blocking(|| {
    heavy_computation()
}).await?;
```

### Documentation (Required)

**All public functions**:

```rust
/// Parses a git diff into individual file changes.
///
/// Handles various diff formats including standard git diff output,
/// diffs with commit hashes, and various path prefixes.
///
/// # Arguments
/// * `diff_content` - Raw git diff text to parse
///
/// # Returns
/// * `Result<Vec<ParsedFile>>` - Parsed file changes
///
/// # Errors
/// Returns error if diff format is unrecognizable or file paths
/// cannot be extracted.
///
/// # Examples
/// ```rust,no_run
/// use git_ai::diff::parse_diff;
///
/// let diff = "diff --git a/file.txt b/file.txt\n...";
/// let files = parse_diff(diff)?;
/// assert!(!files.is_empty());
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn parse_diff(diff_content: &str) -> Result<Vec<ParsedFile>>
```

**Module docs**:

```rust
//! Git diff parsing and processing.
//!
//! Handles parsing git diffs into structured data and
//! processing them with token management for LLM consumption.
```

### Validation Commands

**Must pass before commit**:

```bash
# Individual checks
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
cargo test --all-features
cargo doc --no-deps

# Comprehensive pre-PR check
cargo fmt -- --check && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo test --all-features && \
cargo build --release && \
cargo doc --no-deps

# Watch during development
cargo watch -x test -x clippy
```

### Quality Checklist

When reviewing code:

- [ ] **Clippy**: Zero warnings (`-D warnings`)
- [ ] **Format**: Follows rustfmt.toml exactly
- [ ] **Docs**: All public items with examples
- [ ] **Errors**: Context chains, no unwrap in libs (9 to remove)
- [ ] **Tests**: Pass with coverage
- [ ] **Names**: Clear, descriptive, appropriate length
- [ ] **Performance**: No regressions
- [ ] **Dependencies**: Justified, minimal features

### Code Smells to Avoid

```rust
// ❌ Boolean parameters (use enum)
fn process(diff: &str, fast: bool, cached: bool)

// ✅ Enum for clarity
enum ProcessMode { Fast, Cached, Normal }
fn process(diff: &str, mode: ProcessMode)

// ❌ Mutable statics
static mut COUNTER: usize = 0;

// ✅ LazyLock + Atomic
use std::sync::LazyLock;
static COUNTER: LazyLock<AtomicUsize> = LazyLock::new(|| AtomicUsize::new(0));

// ❌ Stringly-typed APIs
fn set_model(model: &str)  // What strings are valid?

// ✅ Type-safe enums
fn set_model(model: Model)

// ❌ Public fields without builder
pub struct Config { pub api_key: String }

// ✅ Encapsulated with methods
pub struct Config { api_key: String }
impl Config {
    pub fn with_api_key(mut self, key: String) -> Self {
        self.api_key = key;
        self
    }
}
```

### Dependencies (Current Stack)

**From Cargo.toml**:

```toml
# Core
anyhow = "1.0.98"          # Application errors
thiserror = "2.0.12"       # Library errors
tokio = "1.45.1"           # Async runtime
rayon = "1.10.0"           # Data parallelism
futures = "0.3"            # Async utilities
parking_lot = "0.12.3"     # Fast locks

# Git & AI
git2 = "0.20.2"            # Git operations
async-openai = "0.29"      # OpenAI API
tiktoken-rs = "0.7.0"      # Token counting

# Config & Serialization
serde = "1.0"
serde_json = "1.0"
config = "0.15.11"
```

**When adding**:
- ✅ Justify new dependencies
- ✅ Use minimal feature flags
- ✅ Prefer maintained crates
- ❌ Avoid duplicating functionality

### Code Review Output Format

When providing feedback:

```
❌ Line 42: Using unwrap() can panic
✅ Fix: option.ok_or_else(|| anyhow!("no value"))?
📝 Why: Provides better error context and prevents panics
```

---

**Reference**: See [CODE_QUALITY_GUIDE.md](../CODE_QUALITY_GUIDE.md) for complete details.

---

**Remember**: Git AI's goal is to generate meaningful, accurate commit messages that reflect the true intent and impact of changes. Every code change should contribute to this mission while maintaining high code quality standards.
