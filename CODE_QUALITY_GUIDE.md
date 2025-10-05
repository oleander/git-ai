# Git AI Code Quality Guide

> Quality standards for refactoring and new code in the Git AI project

## Overview

This guide defines code quality standards for Git AI based on Rust best practices and the project's existing patterns. Use this as a checklist when reviewing refactoring changes.

---

## 1. Formatting Rules

**Source:** `/rustfmt.toml` (project-specific)

```toml
# Line width and spacing
max_width = 140                    # Wider than default 100
tab_spaces = 2                     # 2-space indentation
hard_tabs = false
edition = "2021"

# Import organization
imports_granularity = "Module"     # Group imports by module
imports_layout = "Horizontal"      # Spread imports horizontally
group_imports = "StdExternalCrate" # std → external → crate
imports_indent = "Block"
reorder_imports = false            # Keep manual ordering
reorder_modules = false

# Function formatting
fn_params_layout = "Compressed"    # Keep params on same line when possible
fn_call_width = 90                 # Wrap function calls at 90 chars
attr_fn_like_width = 120

# Struct formatting
struct_lit_single_line = true      # Single-line structs when possible
struct_lit_width = 50              # Wrap struct literals at 50
use_field_init_shorthand = true    # Use { field } not { field: field }
struct_field_align_threshold = 40

# Other
trailing_comma = "Never"           # No trailing commas
force_multiline_blocks = true
use_small_heuristics = "Max"
chain_width = 60
```

### Verify Formatting

```bash
# Check formatting
cargo fmt -- --check

# Auto-format
cargo fmt
```

---

## 2. Naming Conventions

### Current Patterns (Observed in Codebase)

**Types** - Descriptive multi-word names are acceptable when clear:

```rust
// ✅ Current pattern (acceptable)
pub struct FileAnalysisResult { ... }
pub struct CommitFunctionArgs { ... }
pub struct ParsedFile { ... }
pub enum HookError { ... }

// ⚠️ Consider if refactoring makes sense
pub struct FileDataForScoring { ... }  // Could be FileData with context
pub struct FileWithScore { ... }       // Could be ScoredFile
```

**Functions** - Descriptive verb phrases:

```rust
// ✅ Good
pub fn parse_diff(content: &str) -> Result<Vec<ParsedFile>>
pub fn generate_commit_message(diff: &str) -> Result<String>
pub fn create_analyze_function_tool() -> Result<ChatCompletionTool>

// ❌ Avoid generic names without context
pub fn process() -> Result<()>  // Process what?
pub fn handle() -> Result<()>   // Handle what?
```

**Constants** - SCREAMING_SNAKE_CASE with descriptive names:

```rust
// ✅ Good
const MAX_POOL_SIZE: usize = 1000;
const DEFAULT_STRING_CAPACITY: usize = 8192;
const PARALLEL_CHUNK_SIZE: usize = 25;

// ❌ Too generic
const MAX: usize = 1000;  // Max what?
```

**Module organization** - Use modules to provide context:

```rust
// ✅ Planned structure
mod diff {
    pub struct Parser { ... }
    pub fn parse() -> Result<ParsedFile>
}

mod generation {
    pub struct Strategy { ... }
    pub fn generate() -> Result<String>
}
```

---

## 3. Type System & Patterns

### Derive Macros

**Standard pattern in codebase:**

```rust
// ✅ Common derives (check what's actually needed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysisResult { ... }

#[derive(Error, Debug)]
pub enum HookError { ... }

// ✅ Add derives that make sense
#[derive(Debug, Clone, PartialEq, Eq)]  // Add Eq if implements PartialEq
#[derive(Default)]  // If sensible default exists
```

### Newtype Pattern

```rust
// ✅ Use newtypes for domain modeling
pub struct ImpactScore(f32);
pub struct TokenCount(usize);

// Benefits: Type safety, prevent mixing different numeric types
```

### Exhaustive Enums

```rust
// ✅ Use enums for states
pub enum OperationType {
    Added,
    Modified,
    Deleted,
    Renamed,
    Binary,
}

// ❌ Don't use booleans for multi-state
struct File {
    is_added: bool,
    is_modified: bool,  // What if both are false? Or both true?
}
```

---

## 4. Error Handling

**Current pattern:** Uses both `anyhow` and `thiserror` appropriately

```rust
// ✅ thiserror for library errors (typed)
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HookError {
    #[error("Failed to open repository")]
    OpenRepository,

    #[error("Failed to get patch")]
    GetPatch,

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

// ✅ anyhow for application code
use anyhow::{Context, Result, bail};

pub fn generate(diff: &str) -> Result<String> {
    let parsed = parse_diff(diff)
        .context("Failed to parse git diff")?;  // Add context

    if parsed.is_empty() {
        bail!("Empty diff provided");  // Early return with error
    }

    Ok(result)
}
```

### Reduce unwrap/expect Usage

**Current state:** 9 instances of `unwrap()` or `expect()` across 4 files

```rust
// ❌ Avoid in library code
let value = option.unwrap();
let value = result.expect("this should never fail");

// ✅ Use ? operator
let value = option.ok_or_else(|| anyhow!("No value found"))?;
let value = result.context("Operation failed")?;

// ✅ Or handle explicitly
match result {
    Ok(v) => v,
    Err(e) => {
        log::warn!("Failed to process: {}", e);
        return default_value();
    }
}
```

---

## 5. API Design Principles

### Accept Borrowed, Return Owned

```rust
// ✅ Good
pub fn process(input: &str) -> String
pub fn parse(diff: &str) -> Result<Vec<ParsedFile>>

// ⚠️ Only take ownership if needed
pub fn process(input: String) -> String  // Unnecessary clone for caller
```

### Use Trait Bounds for Flexibility

```rust
// ✅ Accept multiple types
pub fn process(path: impl AsRef<Path>) -> Result<()>
pub fn with_config(config: impl Into<Config>) -> Self

// ❌ Too restrictive
pub fn process(path: &PathBuf) -> Result<()>  // Forces PathBuf
```

### Mark Important Returns

```rust
// ✅ Must-use for important results
#[must_use]
pub fn calculate_impact_score(&self) -> f32

#[must_use = "commit message should be used or logged"]
pub fn generate_commit_message(diff: &str) -> Result<String>
```

---

## 6. Documentation Standards

### Every Public Item Must Have Docs

````rust
/// Parses a git diff into individual file changes.
///
/// Handles various diff formats including standard git diff output,
/// diffs with commit hashes, and various path prefixes (a/, b/, c/, i/).
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
pub fn parse_diff(diff_content: &str) -> Result<Vec<ParsedFile>> {
    // Implementation
}
````

### Module-Level Documentation

````rust
//! Git diff parsing and processing.
//!
//! This module handles parsing git diffs into structured data and
//! processing them with token management for LLM consumption.
//!
//! # Examples
//! ```rust,no_run
//! use git_ai::diff::parse_diff;
//!
//! let files = parse_diff(diff)?;
//! ```

pub mod parser;
pub mod processor;
````

---

## 7. Async Patterns (Tokio)

**Project uses:** `tokio = { version = "1.45.1", features = ["full"] }`

### Structured Concurrency

```rust
// ✅ Use join! for concurrent operations
use futures::future::join_all;

let futures: Vec<_> = files
    .iter()
    .map(|file| analyze_file(client, file))
    .collect();

let results = join_all(futures).await;

// ❌ Avoid spawning unless necessary
for file in files {
    tokio::spawn(analyze_file(client, file)); // Loses structured control
}
```

### Error Handling in Async

```rust
// ✅ Handle errors at each layer
pub async fn generate(diff: &str) -> Result<String> {
    let parsed = parse_diff(diff)?;  // Sync error

    let analysis = analyze_files(&parsed)
        .await
        .context("Analysis failed")?;  // Async error with context

    Ok(format_result(analysis))
}
```

---

## 8. Performance Patterns

### Pre-allocate Capacity

```rust
// ✅ Found in codebase
let mut files = HashMap::with_capacity(ESTIMATED_FILES_COUNT);
let mut results = Vec::with_capacity(total_files);
let mut buffer = String::with_capacity(DEFAULT_STRING_CAPACITY);

// ❌ Wasteful reallocations
let mut files = HashMap::new();  // Will grow multiple times
```

### Use Iterators

```rust
// ✅ Iterator chains
let scored: Vec<_> = files
    .into_iter()
    .filter(|f| f.lines_changed > 0)
    .map(|f| calculate_score(f))
    .collect();

// ⚠️ Loops when iterators are clearer
let mut scored = Vec::new();
for f in files {
    if f.lines_changed > 0 {
        scored.push(calculate_score(f));
    }
}
```

### Inline Hot Paths

```rust
// ✅ Inline small, frequently-called functions
#[inline]
pub fn calculate_single_score(data: &FileData) -> f32 {
    // Small function called many times in tight loop
}

// ❌ Don't inline large functions or cold paths
#[inline]  // Bloats binary for no benefit
pub async fn generate_commit_message(diff: &str) -> Result<String> {
    // 100+ lines of complex logic
}
```

---

## 9. Parallel Processing (Rayon)

**Project uses:** `rayon = "1.10.0"`

```rust
use rayon::prelude::*;

// ✅ Parallel iterator for CPU-bound work
let results: Vec<_> = files
    .par_iter()  // Parallel iterator
    .map(|file| process_file(file))
    .collect();

// ✅ Chunking for better performance
let chunks: Vec<_> = files
    .chunks(PARALLEL_CHUNK_SIZE)
    .map(|chunk| chunk.to_vec())
    .collect();

chunks.par_iter()
    .try_for_each(|chunk| process_chunk(chunk))?;
```

---

## 10. Testing Standards

### Test Organization

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_file() {
        let diff = "diff --git a/file.txt b/file.txt\n...";
        let result = parse_diff(diff).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn test_async_generation() {
        let result = generate_commit_message("test diff").await;
        assert!(result.is_ok());
    }
}
```

### Property-Based Tests (Future)

```rust
// TODO: Add proptest for invariants
use proptest::prelude::*;

proptest! {
    #[test]
    fn truncate_never_exceeds_limit(
        text in "\\PC*",
        limit in 1..1000usize
    ) {
        let model = Model::GPT4oMini;
        let result = model.truncate(&text, limit).unwrap();
        let tokens = model.count_tokens(&result).unwrap();
        assert!(tokens <= limit);
    }
}
```

---

## 11. Dependencies

### Current Stack (Reference)

```toml
# Core
anyhow = "1.0.98"           # Application errors
thiserror = "2.0.12"        # Library errors
tokio = "1.45.1"            # Async runtime
futures = "0.3"             # Async utilities
parking_lot = "0.12.3"      # Better Mutex/RwLock

# Git
git2 = "0.20.2"             # Git operations

# OpenAI
async-openai = "0.29"       # OpenAI API client
tiktoken-rs = "0.7.0"       # Token counting

# Parallelism
rayon = "1.10.0"            # Data parallelism
num_cpus = "1.16.0"         # CPU detection

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Adding Dependencies

- ✅ Justify new dependencies
- ✅ Use minimal feature flags
- ✅ Prefer maintained crates
- ❌ Avoid duplicating functionality

---

## 12. Code Smells & Anti-Patterns

### Avoid These

```rust
// ❌ Boolean parameters (use enum)
fn process(diff: &str, is_fast: bool, is_cached: bool)

// ✅ Use enum for clarity
enum ProcessMode {
    Fast,
    Cached,
    Normal,
}
fn process(diff: &str, mode: ProcessMode)

// ❌ Mutable statics
static mut COUNTER: usize = 0;

// ✅ Use OnceCell or lazy_static
use std::sync::LazyLock;
static COUNTER: LazyLock<AtomicUsize> = LazyLock::new(|| AtomicUsize::new(0));

// ❌ Stringly-typed APIs
fn set_model(model: &str)  // What strings are valid?

// ✅ Use enums
fn set_model(model: Model)

// ❌ Public fields without builder
pub struct Config {
    pub api_key: String,
    pub model: String,
}

// ✅ Use builder or methods
pub struct Config {
    api_key: String,
    model: String,
}

impl Config {
    pub fn with_api_key(mut self, key: String) -> Self {
        self.api_key = key;
        self
    }
}
```

---

## 13. Pre-Commit Checklist

Before committing refactored code, verify:

### Automated Checks

```bash
# Format check
cargo fmt -- --check

# Linting (should pass with zero warnings)
cargo clippy --all-targets --all-features -- -D warnings

# Tests
cargo test --all-features

# Build release
cargo build --release

# Documentation
cargo doc --no-deps
```

### Manual Review

- [ ] All public functions have documentation with examples
- [ ] No `unwrap()` or `expect()` in library code (or justified)
- [ ] Error messages are helpful and actionable
- [ ] Types and functions have clear, descriptive names
- [ ] Code follows rustfmt.toml formatting
- [ ] No clippy warnings
- [ ] Tests pass
- [ ] Performance hasn't regressed (if applicable)

---

## 14. Issue-Specific Guidelines

### When Refactoring

1. **Make small commits** - One logical change per commit
2. **Test continuously** - Run tests after each change
3. **Preserve behavior** - Refactoring should not change functionality
4. **Document decisions** - Add comments explaining _why_, not _what_

### When Adding Features

1. **Design API first** - Think about usage before implementation
2. **Write tests first** - TDD when possible
3. **Document as you go** - Don't defer documentation
4. **Consider performance** - Profile if changes affect hot paths

---

## 15. Resources

### Tools

- **Format:** [`cargo fmt`](https://github.com/rust-lang/rustfmt)
- **Lint:** [`cargo clippy`](https://github.com/rust-lang/rust-clippy)
- **Audit:** [`cargo audit`](https://github.com/rustsec/rustsec)
- **Coverage:** [`cargo tarpaulin`](https://github.com/xd009642/tarpaulin)

### References

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Effective Rust](https://www.lurklurk.org/effective-rust/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Rayon Documentation](https://docs.rs/rayon/)

---

## Version

- **Created:** 2025-10-05
- **Based on:** Git AI codebase as of v1.0.9
- **Last Updated:** 2025-10-05

**Note:** This guide evolves with the project. Update it when patterns change.
