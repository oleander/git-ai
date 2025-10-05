# Git AI Code Quality Guide

> **Cross-referenced with actual codebase** - All rules verified against Git AI v1.0.9

---

## Formatting (rustfmt.toml - Mandatory)

**Source**: `/rustfmt.toml` - Enforced by CI

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

---

## Naming Conventions

### Types

**Descriptive, clear names** (multi-word acceptable):

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

### Functions

**Verb phrases, context-appropriate length**:

```rust
// ✅ Descriptive verbs (from codebase)
fn parse_diff(content: &str) -> Result<Vec<ParsedFile>>
fn generate_commit_message(diff: &str) -> Result<String>
fn create_analyze_function_tool() -> Result<ChatCompletionTool>

// ❌ Generic without context
fn process() -> Result<()>
fn handle() -> Result<()>
```

### Constants

**SCREAMING_SNAKE_CASE, descriptive**:

```rust
// ✅ From actual codebase
const MAX_POOL_SIZE: usize = 1000;
const DEFAULT_STRING_CAPACITY: usize = 8192;
const PARALLEL_CHUNK_SIZE: usize = 25;
```

### Modules

**Scope types, enable short names within**:

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

---

## Type System

### Newtypes for Domain Modeling

```rust
struct ImpactScore(f32);
struct TokenCount(usize);
struct UserId(i64);
```

### Exhaustive Enums Over Booleans

```rust
// ✅ From codebase
enum OperationType { Added, Modified, Deleted, Renamed, Binary }

// ❌ Anti-pattern
struct File { is_added: bool, is_modified: bool }
```

### Typestate Pattern

```rust
struct Unvalidated;
struct Validated;
struct Email<S = Unvalidated> {
    value: String,
    _state: PhantomData<S>
}
```

### Standard Derives

**Pattern from codebase**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Error, Debug)]
#[derive(Default)]  // When sensible default exists
#[derive(PartialEq, Eq)]  // Add Eq if PartialEq
```

---

## Error Handling

### thiserror for Libraries

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

### anyhow for Applications

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

### Eliminate unwrap/expect

**Current issue**: 9 instances across 4 files - should be reduced

```rust
// ❌ Avoid in production code
let value = option.unwrap();
let value = result.expect("this should never fail");

// ✅ Use ? operator with context
let value = option.ok_or_else(|| anyhow!("no value found"))?;
let value = result.context("operation failed")?;
```

---

## API Design

### Accept Borrowed, Return Owned

```rust
// ✅ From codebase pattern
fn process(input: &str) -> String
fn parse(diff: &str) -> Result<Vec<ParsedFile>>
```

### Trait Bounds for Flexibility

```rust
fn process(path: impl AsRef<Path>) -> Result<()>
fn with_config(config: impl Into<Config>) -> Self
```

### Must-Use for Important Returns

```rust
#[must_use = "commit message should be used or logged"]
pub fn generate_commit_message(diff: &str) -> Result<String>
```

---

## Documentation (Required)

### All Public Functions

````rust
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
````

### Module-Level Docs

````rust
//! Git diff parsing and processing.
//!
//! Handles parsing git diffs into structured data and
//! processing them with token management for LLM consumption.
//!
//! # Examples
//! ```rust,no_run
//! use git_ai::diff::parse_diff;
//! let files = parse_diff(diff)?;
//! ```
````

---

## Performance

### Pre-allocate Capacity

**Pattern from codebase**:

```rust
// ✅ Always pre-allocate when size is known
let mut files = HashMap::with_capacity(ESTIMATED_FILES_COUNT);
let mut results = Vec::with_capacity(total_files);
let mut buffer = String::with_capacity(DEFAULT_STRING_CAPACITY);
```

### Iterators Over Loops

```rust
// ✅ Iterator chains
let scored: Vec<_> = files
    .into_iter()
    .filter(|f| f.lines_changed > 0)
    .map(calculate_score)
    .collect();
```

### Inline Hot Paths

```rust
// ✅ Inline small, frequently-called functions
#[inline]
pub fn calculate_score(data: &FileData) -> f32 { /* ... */ }

// ❌ Don't inline large functions
#[inline]  // Bloats binary
pub async fn generate_commit_message(diff: &str) -> Result<String> {
    // 100+ lines
}
```

### Parallel Processing (Rayon)

**Heavy usage in codebase** - `rayon = "1.10.0"`:

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

---

## Async (Tokio 1.45.1)

### Structured Concurrency

**Pattern from multi_step_integration.rs**:

```rust
use futures::future::join_all;

// ✅ Structured concurrency
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

### spawn_blocking for CPU Work

```rust
let result = tokio::task::spawn_blocking(|| {
    heavy_computation()
}).await?;
```

---

## Code Smells

### Anti-Patterns to Eliminate

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
```

---

## Validation

### Pre-Commit Commands

```bash
# Must pass before commit
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

# Watch mode during development
cargo watch -x test -x clippy
```

---

## Quality Checklist

When reviewing code:

- [ ] **Clippy**: Zero warnings (`-D warnings`)
- [ ] **Format**: Follows rustfmt.toml exactly
- [ ] **Docs**: All public items with examples
- [ ] **Errors**: Context chains, minimize unwrap usage in library code
- [ ] **Tests**: Pass with coverage
- [ ] **Names**: Clear, descriptive, appropriate length
- [ ] **Performance**: No regressions
- [ ] **Dependencies**: Justified, minimal features

---

## Dependencies (Current Stack)

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

---

## Code Review Format

When providing feedback:

```
❌ Line 42: Using unwrap() can panic
✅ Fix: option.ok_or_else(|| anyhow!("no value"))?
📝 Why: Provides better error context and prevents panics

❌ Line 87: Boolean parameter unclear
✅ Fix: enum ProcessMode { Fast, Normal }
📝 Why: Makes API self-documenting and extensible
```

---

## Resources

### Tools

- [`cargo fmt`](https://github.com/rust-lang/rustfmt) - Auto-formatting
- [`cargo clippy`](https://github.com/rust-lang/rust-clippy) - Linting
- [`cargo audit`](https://github.com/rustsec/rustsec) - Security audit
- [`cargo tarpaulin`](https://github.com/xd009642/tarpaulin) - Coverage

### References

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Effective Rust](https://www.lurklurk.org/effective-rust/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Rayon Docs](https://docs.rs/rayon/)

---

**Version**: 1.1 | **Updated**: 2025-10-05 | **Based on**: Git AI v1.0.9
