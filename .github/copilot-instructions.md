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
git-ai config set <key> <value>  # Set configuration
git-ai config get <key>          # Get configuration value
git-ai config reset              # Reset to defaults
git-ai hook install              # Install git hook
git-ai hook uninstall            # Remove git hook
git-ai hook reinstall            # Reinstall hook
```

## Development Workflow

### Local Development

- Use `just local-install` for quick dev installation
- Run `cargo test` before committing
- Check `./scripts/integration-tests` for comprehensive testing
- Run `./scripts/hook-stress-test` for hook testing

### Building

- Release builds use LTO and aggressive optimization
- Include debug symbols even in release builds
- Multi-target support: Linux (GNU/musl), macOS

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

#[derive(Serialize, Deserialize)]
struct FileAnalysis {
    lines_added: u32,
    lines_removed: u32,
    category: FileCategory,
    summary: String,
}

// Use with OpenAI function calling for structured outputs
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

let analyses: Vec<FileAnalysis> = files
    .par_iter()
    .map(|file| analyze_file(file))
    .collect();
```

## Workspace Rules

The project includes two important workspace rules:

1. **cleanup**: Full repository cleanup for Rust projects (remove dead code, unused dependencies, outdated docs)
2. **structure**: Enforce nested module file structure (split underscored filenames into directory-based modules)

Follow these rules when refactoring or adding new code.

---

**Remember**: Git AI's goal is to generate meaningful, accurate commit messages that reflect the true intent and impact of changes. Every code change should contribute to this mission.
