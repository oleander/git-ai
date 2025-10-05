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

**Current issue**: 9 instances across 4 files - should be reduced

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
