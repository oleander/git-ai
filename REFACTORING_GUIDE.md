# Git AI Codebase Cleanup and Refactoring Guide

## Executive Summary

This guide provides comprehensive instructions for improving the maintainability, consistency, and quality of the Git AI codebase. The refactoring focuses on code organization, naming conventions, removing technical debt, improving architecture, and enhancing documentation.

**Estimated Impact:** High - These changes will significantly improve code maintainability and reduce cognitive load for future development.

---

## Table of Contents

1. [Code Organization & Structure](#1-code-organization--structure)
2. [Naming Conventions & Consistency](#2-naming-conventions--consistency)
3. [Technical Debt Removal](#3-technical-debt-removal)
4. [Architecture Improvements](#4-architecture-improvements)
5. [Performance Optimization](#5-performance-optimization)
6. [Error Handling Enhancement](#6-error-handling-enhancement)
7. [Documentation Standards](#7-documentation-standards)
8. [Testing Improvements](#8-testing-improvements)

---

## 1. Code Organization & Structure

### 1.1 Module Restructuring

**Problem:** Large monolithic files with mixed responsibilities

**Files to Refactor:**

- `src/hook.rs` (725 lines) - Too large, mixing diff processing, traits, and parallel logic
- Multiple overlapping multi-step modules (`multi_step_analysis.rs`, `multi_step_integration.rs`, `simple_multi_step.rs`)

**Action Items:**

```
src/
├── diff/                          # NEW: Diff processing module
│   ├── mod.rs                     # Re-exports and public API
│   ├── parser.rs                  # Git diff parsing (from hook.rs)
│   ├── processor.rs               # Diff processing traits (PatchDiff, etc.)
│   ├── optimization.rs            # Parallel processing & token management
│   └── traits.rs                  # FilePath, Utf8String, DiffDeltaPath
├── generation/                    # NEW: Commit message generation
│   ├── mod.rs
│   ├── multi_step.rs              # Merge multi_step_* modules
│   ├── fallback.rs                # Centralized fallback logic
│   ├── local.rs                   # Local generation strategies
│   └── api.rs                     # API-based generation
├── api/                           # NEW: External API integrations
│   ├── mod.rs
│   ├── openai.rs                  # Refactored from src/openai.rs
│   ├── ollama.rs                  # Keep as-is
│   └── client.rs                  # HTTP client wrapper
└── core/                          # Existing core functionality
    ├── config.rs
    ├── model.rs
    ├── filesystem.rs
    └── profiling.rs
```

**Specific Actions:**

1. **Create `diff/` module:**
   - Extract diff parsing from `hook.rs` lines 212-402 → `diff/parser.rs`
   - Move `PatchDiff` trait and implementation → `diff/processor.rs`
   - Extract parallel processing logic (`process_chunk`) → `diff/optimization.rs`
   - Move utility traits (`FilePath`, `Utf8String`, `DiffDeltaPath`) → `diff/traits.rs`

2. **Consolidate multi-step modules:**
   - Merge `multi_step_analysis.rs`, `multi_step_integration.rs`, and `simple_multi_step.rs`
   - Create single `generation/multi_step.rs` with clear phases:
     ```rust
     pub mod analysis;    // File analysis functions
     pub mod scoring;     // Impact scoring
     pub mod candidates;  // Message candidate generation
     pub mod selection;   // Best candidate selection
     ```

3. **Centralize fallback logic:**
   - Currently scattered across `commit.rs`, `openai.rs`, and multi-step modules
   - Create `generation/fallback.rs` with a clear strategy pattern:

     ```rust
     pub enum GenerationStrategy {
         MultiStepAPI,
         LocalMultiStep,
         SingleStepAPI,
     }

     pub async fn generate_with_fallback(
         diff: &str,
         strategies: Vec<GenerationStrategy>
     ) -> Result<String>
     ```

### 1.2 Remove Dead Code Markers

**Problem:** `#![allow(dead_code)]` in `src/hook.rs` and `src/bin/hook.rs` masks actual dead code

**Actions:**

1. Remove `#![allow(dead_code)]` from both files
2. Run `cargo clippy` and address ALL dead code warnings:
   - Delete truly unused code
   - Mark intentionally public API with proper documentation
   - Add `#[allow(dead_code)]` only to specific items with explanatory comments if needed

3. **Specifically investigate:**
   - `StringPool` struct in `hook.rs` (lines 62-85) - appears to be defined but never used
   - `Args` struct in `hook.rs` (lines 50-58) - might be redundant with bin/hook.rs
   - Test helper functions that may be obsolete

---

## 2. Naming Conventions & Consistency

### 2.1 Type Naming Inconsistencies

**Problem:** Same concepts named differently across the codebase

**Inconsistencies Found:**

| Current Names                                                 | Issue                                  | Standardize To                                     |
| ------------------------------------------------------------- | -------------------------------------- | -------------------------------------------------- |
| `App` / `Settings`                                            | Used interchangeably for configuration | `Config` or `AppConfig`                            |
| `Response` (openai.rs) / `CommitFunctionArgs`                 | Both represent commit responses        | `CommitResponse`                                   |
| `FileAnalysisResult` / `FileWithScore` / `FileDataForScoring` | Three similar structs                  | Consolidate to `AnalyzedFile` with optional fields |
| `ParsedFile` / `FileChange`                                   | Similar file representation            | Choose one: `FileChange`                           |

**Actions:**

1. **Standardize Configuration:**

   ```rust
   // In config.rs - Rename App to AppConfig
   pub struct AppConfig {
       pub openai_api_key: Option<String>,
       // ... rest
   }

   // Update all references:
   // config::APP → config::APP_CONFIG
   // App::new() → AppConfig::new()
   // Settings → AppConfig (remove alias)
   ```

2. **Unify File Representations:**

   ```rust
   // In generation/types.rs (new file)
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct FileChange {
       pub path: String,
       pub operation: OperationType,
       pub diff_content: Option<String>,

       // Analysis fields (optional, filled after analysis)
       pub lines_added: Option<u32>,
       pub lines_removed: Option<u32>,
       pub category: Option<FileCategory>,
       pub summary: Option<String>,
       pub impact_score: Option<f32>,
   }

   #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
   pub enum OperationType {
       Added, Modified, Deleted, Renamed, Binary
   }

   #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
   pub enum FileCategory {
       Source, Test, Config, Docs, Binary, Build
   }
   ```

3. **Consolidate Response Types:**
   ```rust
   // Single commit response type
   pub struct CommitResponse {
       pub message: String,
       pub reasoning: String,
       pub files: HashMap<String, FileChange>,
   }
   ```

### 2.2 Function Naming Consistency

**Problem:** Inconsistent verb prefixes and naming patterns

**Examples:**

- `generate_commit_message()` vs `create_commit_function_tool()`
- `get_instruction_template()` vs `token_used()`
- `call_analyze_function()` vs `analyze_file()`

**Actions:**

1. **Standardize function verb prefixes:**
   - **create\_**: Building new objects/structures
   - **generate\_**: Producing text/messages
   - **parse\_**: Converting from one format to another
   - **analyze\_**: Examining and extracting insights
   - **calculate\_**: Computing numerical values
   - **fetch\_** / **get\_**: Retrieving existing data

2. **Apply consistently:**

   ```rust
   // Before: get_instruction_template()
   // After:  generate_instruction_template()

   // Before: token_used()
   // After:  calculate_token_usage() or get_token_usage()

   // Before: call_analyze_function()
   // After:  analyze_file_via_api()
   ```

### 2.3 Module and File Naming

**Problem:** Inconsistent module prefixes

**Current State:**

- `multi_step_analysis.rs`, `multi_step_integration.rs` - prefixed
- `simple_multi_step.rs` - different prefix pattern
- `debug_output.rs` - descriptive name
- `function_calling.rs` - descriptive name

**Actions:**

1. **Remove redundant prefixes** when inside a parent module:

   ```
   # Before
   src/multi_step_analysis.rs
   src/multi_step_integration.rs

   # After
   src/generation/
       ├── mod.rs
       ├── analysis.rs      # was multi_step_analysis.rs
       ├── integration.rs   # was multi_step_integration.rs
       └── fallback.rs      # was simple_multi_step.rs
   ```

2. **Use descriptive names** reflecting purpose, not implementation:
   - `function_calling.rs` → `api_tools.rs` or `openai_tools.rs`
   - `debug_output.rs` → `diagnostics.rs` or keep as-is

---

## 3. Technical Debt Removal

### 3.1 TODO Comments

**Problem:** Multiple TODO comments indicating incomplete work

**Found in:**

- `src/model.rs` lines 14, 25, 214, 242, 249, 262
- Other files (search for `TODO:` pattern)

**Actions:**

1. **Audit all TODO comments:**

   ```bash
   grep -r "TODO" src/ --include="*.rs"
   ```

2. **For each TODO:**
   - **If trivial:** Implement immediately
   - **If complex:** Create GitHub issue and reference it in comment
   - **If obsolete:** Remove the comment

3. **Specific TODOs to address:**

   ```rust
   // model.rs:14 - Commented out import
   // Decision: Remove commented code or uncomment and use

   // model.rs:25 - "Get this from config.rs"
   // Solution: Create shared constants module
   const DEFAULT_MODEL: &str = config::DEFAULT_MODEL;

   // model.rs:214 - "This should be based on the model string"
   // Solution: Implement model-specific tokenizer selection
   fn get_tokenizer(model_str: &str) -> CoreBPE {
       match model_str {
           "gpt-4" | "gpt-4o" | "gpt-4o-mini" | "gpt-4.1" => {
               tiktoken_rs::cl100k_base()
           }
           _ => tiktoken_rs::cl100k_base() // fallback
       }.expect("Failed to create tokenizer")
   }
   ```

### 3.2 Commented-Out Code

**Problem:** Commented code clutters files and creates confusion

**Found in:**

- `model.rs`: `// use crate::config::format_prompt;` (line 14)
- `model.rs`: Multiple commented lines around template/prompt handling (lines 224, 243)

**Actions:**

1. **Remove all commented-out code** unless:
   - It's a temporary development comment (should be in a branch, not main)
   - It has a clear explanation of why it's there

2. **For legitimate reasons to keep commented code:**
   ```rust
   // INTENTIONAL: Keeping this as reference for migration to v2
   // Issue: #123
   // Remove after: 2025-Q2
   // let old_approach = calculate_score_v1();
   ```

### 3.3 Unused Imports and Dependencies

**Problem:** Potentially unused imports increasing compilation time

**Actions:**

1. **Run automated cleanup:**

   ```bash
   cargo fix --allow-dirty --allow-staged
   cargo clippy --fix --allow-dirty --allow-staged
   ```

2. **Audit dependencies in Cargo.toml:**
   - Check if all dependencies are actually used
   - Consider removing or making optional:
     - `mustache` - used only for simple template (could use format!)
     - `maplit` - used for single hashmap! macro
     - `textwrap` - check if actually used
     - `console` - check if fully utilized

3. **Run dependency audit:**
   ```bash
   cargo tree --duplicates
   cargo udeps  # requires: cargo install cargo-udeps
   ```

### 3.4 Duplicate Logic

**Problem:** Similar code patterns duplicated across multiple files

**Instances Found:**

1. **Diff file extraction logic duplicated:**
   - `openai.rs` lines 56-75 (simple extraction)
   - `multi_step_integration.rs` lines 212-402 (full parser)
   - `simple_multi_step.rs` lines 77-86 (basic extraction)

   **Solution:** Centralize in `diff/parser.rs`

2. **Commit message generation logic scattered:**
   - `commit.rs` - orchestration
   - `openai.rs` - API calls with generation
   - `multi_step_integration.rs` - multi-step generation
   - `simple_multi_step.rs` - simplified generation

   **Solution:** Single source of truth in `generation/` module

3. **API key validation duplicated:**
   - `commit.rs` lines 86-102
   - `openai.rs` lines 113-126

   **Solution:** Create `api/auth.rs` with validation helper

---

## 4. Architecture Improvements

### 4.1 Fallback Strategy Pattern

**Problem:** Fallback logic is scattered and hard to follow

**Current State:**

- `commit.rs` has primary fallback orchestration
- `openai.rs` has its own fallback
- Multiple places check API key validity

**Solution - Implement Strategy Pattern:**

```rust
// In generation/fallback.rs

pub trait GenerationStrategy: Send + Sync {
    async fn generate(
        &self,
        diff: &str,
        config: &GenerationConfig,
    ) -> Result<String>;

    fn name(&self) -> &str;
}

pub struct MultiStepAPIStrategy {
    client: Client<OpenAIConfig>,
}

impl GenerationStrategy for MultiStepAPIStrategy {
    async fn generate(&self, diff: &str, config: &GenerationConfig) -> Result<String> {
        // Implementation from multi_step_integration.rs
    }

    fn name(&self) -> &str { "Multi-step API" }
}

pub struct LocalMultiStepStrategy;

impl GenerationStrategy for LocalMultiStepStrategy {
    async fn generate(&self, diff: &str, config: &GenerationConfig) -> Result<String> {
        // Implementation from current local generation
    }

    fn name(&self) -> &str { "Local multi-step" }
}

pub struct SingleStepAPIStrategy {
    client: Client<OpenAIConfig>,
}

impl GenerationStrategy for SingleStepAPIStrategy {
    async fn generate(&self, diff: &str, config: &GenerationConfig) -> Result<String> {
        // Current single-step implementation
    }

    fn name(&self) -> &str { "Single-step API" }
}

/// Orchestrates generation with automatic fallback
pub async fn generate_with_fallback(
    diff: &str,
    config: &GenerationConfig,
) -> Result<String> {
    let strategies: Vec<Box<dyn GenerationStrategy>> = vec![
        Box::new(MultiStepAPIStrategy::new(config)?),
        Box::new(LocalMultiStepStrategy),
        Box::new(SingleStepAPIStrategy::new(config)?),
    ];

    let mut last_error = None;

    for strategy in strategies {
        log::info!("Attempting generation with: {}", strategy.name());

        match strategy.generate(diff, config).await {
            Ok(message) => {
                log::info!("Successfully generated with: {}", strategy.name());
                return Ok(message);
            }
            Err(e) => {
                log::warn!("Failed with {}: {}", strategy.name(), e);
                last_error = Some(e);

                // Don't retry on auth errors
                if e.to_string().contains("invalid_api_key") {
                    return Err(e);
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("All generation strategies failed")))
}
```

### 4.2 Configuration Management

**Problem:** Configuration is accessed globally through lazy_static, making testing difficult

**Current Issues:**

- `config::APP` global static
- Hard to mock for testing
- Settings passed around inconsistently

**Solution - Dependency Injection:**

```rust
// In config.rs

/// Configuration that can be injected
pub struct AppConfig {
    pub openai_api_key: Option<String>,
    pub model: Option<String>,
    pub max_tokens: Option<usize>,
    pub max_commit_length: Option<usize>,
    pub timeout: Option<usize>,
}

impl AppConfig {
    /// Load from default locations
    pub fn load() -> Result<Self> {
        // Current implementation
    }

    /// Create for testing
    #[cfg(test)]
    pub fn test_config() -> Self {
        Self {
            openai_api_key: Some("test-key".to_string()),
            model: Some("gpt-4o-mini".to_string()),
            max_tokens: Some(2024),
            max_commit_length: Some(72),
            timeout: Some(30),
        }
    }
}

/// Global config for convenience (but discourage overuse)
pub static APP_CONFIG: LazyLock<AppConfig> = LazyLock::new(|| {
    AppConfig::load().expect("Failed to load config")
});
```

**Update All Functions to Accept Config:**

```rust
// Before
pub async fn generate(patch: String, remaining_tokens: usize, model: Model,
                      settings: Option<&Settings>) -> Result<Response>

// After
pub async fn generate(
    diff: &str,
    config: &AppConfig,
    model: &Model,
) -> Result<CommitResponse>
```

### 4.3 Error Handling Consolidation

**Problem:** Mixed error handling approaches

**Current State:**

- `HookError` enum in hook.rs
- Direct `anyhow::Error` usage everywhere else
- Some `thiserror` usage

**Solution - Consistent Error Hierarchy:**

```rust
// In error.rs (new file)

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitAiError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Git repository error: {0}")]
    Git(#[from] git2::Error),

    #[error("API error: {0}")]
    Api(#[from] ApiError),

    #[error("Parsing error: {0}")]
    Parse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("API request failed: {0}")]
    RequestFailed(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, GitAiError>;
```

---

## 5. Performance Optimization

### 5.1 Token Counting and Truncation

**Problem:** Complex, potentially slow token counting with multiple strategies

**Current Issues:**

- Multiple truncation methods
- Recursive walking with binary search
- Character-based estimation that may be inaccurate

**Actions:**

1. **Simplify truncation logic in model.rs:**

   ```rust
   pub fn truncate(&self, text: &str, max_tokens: usize) -> Result<String> {
       let current_tokens = self.count_tokens(text)?;
       if current_tokens <= max_tokens {
           return Ok(text.to_string());
       }

       // Binary search on character boundaries for efficiency
       let chars: Vec<char> = text.chars().collect();
       let mut low = 0;
       let mut high = chars.len();
       let mut best_end = 0;

       while low < high {
           let mid = (low + high + 1) / 2;
           let candidate: String = chars[..mid].iter().collect();
           let tokens = self.count_tokens(&candidate)?;

           if tokens <= max_tokens {
               best_end = mid;
               low = mid;
           } else {
               high = mid - 1;
           }
       }

       Ok(chars[..best_end].iter().collect())
   }
   ```

2. **Remove unused `walk_truncate` method** - the recursive approach adds complexity

3. **Cache tokenizer initialization** - already done with `OnceLock`, but verify it's used consistently

### 5.2 Parallel Processing Review

**Problem:** Complex parallel processing with multiple optimization paths

**In hook.rs lines 189-359:**

- Multiple fast paths for different diff sizes
- Adaptive chunk sizing
- Complex token allocation

**Actions:**

1. **Profile actual usage patterns:**
   - Log statistics: diff sizes, file counts, processing times
   - Determine which optimizations provide real value

2. **Simplify if justified:**

   ```rust
   // Consider: Does the complexity of 5 fast paths justify maintenance?
   // Profiles will show if users typically have <5 files or >500 files
   // If most are in the middle, simplify to:

   pub fn to_patch(&self, max_tokens: usize, model: Model) -> Result<String> {
       let files = self.collect_diff_data()?;
       if files.is_empty() {
           return Ok(String::new());
       }

       // Single unified path with good defaults
       process_files_optimized(files, max_tokens, model)
   }
   ```

3. **Document performance characteristics:**
   - Add doc comments explaining the tradeoffs
   - Include benchmarks or expected usage patterns

### 5.3 StringPool Usage

**Problem:** `StringPool` defined but never used in hook.rs

**Actions:**

1. **Determine if intentional optimization:**
   - Check git history for when it was added
   - See if it was part of an incomplete optimization

2. **Either implement or remove:**

   ```rust
   // If keeping, use it:
   thread_local! {
       static STRING_POOL: RefCell<StringPool> =
           RefCell::new(StringPool::new(DEFAULT_STRING_CAPACITY));
   }

   // Use in collect_diff_data and elsewhere

   // OR remove entirely if benchmarks show no benefit
   ```

---

## 6. Error Handling Enhancement

### 6.1 API Key Validation Centralization

**Problem:** API key validation logic duplicated in multiple files

**Actions:**

```rust
// In api/auth.rs (new file)

/// Validates an API key is present and valid
pub fn validate_api_key(key: Option<&str>) -> Result<&str> {
    match key {
        None => bail!("OpenAI API key not configured"),
        Some(k) if k.is_empty() || k == "<PLACE HOLDER FOR YOUR API KEY>" => {
            bail!("Invalid or placeholder API key")
        }
        Some(k) => Ok(k),
    }
}

/// Gets API key from config or environment
pub fn get_api_key(config: &AppConfig) -> Result<String> {
    // Try config first
    if let Some(key) = &config.openai_api_key {
        validate_api_key(Some(key))?;
        return Ok(key.clone());
    }

    // Try environment variable
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        validate_api_key(Some(&key))?;
        return Ok(key);
    }

    bail!(
        "OpenAI API key not found. Set via:\n\
         1. git-ai config set openai-api-key <key>\n\
         2. OPENAI_API_KEY environment variable"
    )
}
```

**Update all API key checks to use this function.**

### 6.2 Better Error Context

**Problem:** Some errors lack helpful context

**Example Issues:**

- Generic "Failed to analyze files" without details
- Lost error chains in fallback logic

**Actions:**

1. **Add context to all error propagation:**

   ```rust
   // Before
   let parsed = parse_diff(diff)?;

   // After
   let parsed = parse_diff(diff)
       .context("Failed to parse git diff")?;
   ```

2. **Preserve error chains in fallbacks:**
   ```rust
   // In fallback orchestration
   match strategy.generate(diff, config).await {
       Err(e) => {
           let context_err = anyhow::Error::from(e)
               .context(format!("Strategy '{}' failed", strategy.name()));
           errors.push(context_err);
       }
   }
   ```

---

## 7. Documentation Standards

### 7.1 Module-Level Documentation

**Problem:** Most modules lack top-level documentation

**Required for All Modules:**

````rust
//! Brief description of module purpose
//!
//! # Overview
//! More detailed explanation of what this module does and why it exists.
//!
//! # Examples
//! ```rust,no_run
//! use git_ai::generation::generate_commit_message;
//!
//! let message = generate_commit_message(diff, config).await?;
//! ```
//!
//! # Architecture
//! Explain how this fits into the larger system.
````

**Apply to All Modules, Especially:**

- `hook.rs` → needs explanation of diff processing architecture
- `multi_step_*.rs` → needs explanation of multi-step algorithm
- `model.rs` → needs explanation of token management strategy

### 7.2 Function Documentation

**Problem:** Public functions lack documentation

**Standard for Public Functions:**

````rust
/// Brief one-line description
///
/// More detailed explanation of what the function does, its algorithm,
/// and any important considerations.
///
/// # Arguments
/// * `diff` - The git diff to process
/// * `config` - Application configuration
///
/// # Returns
/// * `Result<String>` - Generated commit message or error
///
/// # Errors
/// Returns error if:
/// - Diff is empty or malformed
/// - API key is invalid
/// - All generation strategies fail
///
/// # Examples
/// ```rust,no_run
/// let message = generate_commit_message(&diff, &config).await?;
/// assert!(!message.is_empty());
/// ```
pub async fn generate_commit_message(
    diff: &str,
    config: &AppConfig,
) -> Result<String>
````

**Priority Functions to Document:**

- All public functions in `commit.rs`
- All public functions in `openai.rs`
- All public functions in `multi_step_integration.rs`
- All trait methods

### 7.3 Code Comments

**Problem:** Complex logic lacks explanatory comments

**Where to Add Comments:**

1. **Complex algorithms:**

   ```rust
   // Binary search to find optimal truncation point.
   // We iterate on character positions (not bytes) to ensure we
   // never split a UTF-8 character. Each iteration narrows the
   // range by half until we find the largest string that fits.
   while low < high {
       // ...
   }
   ```

2. **Non-obvious optimizations:**

   ```rust
   // Fast path: Skip token counting for small diffs.
   // For ≤5 files, character-based estimation is accurate enough
   // and 10x faster than proper tokenization.
   if files.len() <= 5 && max_tokens > 500 {
       // ...
   }
   ```

3. **Business logic decisions:**
   ```rust
   // Only generate messages for commits without user-provided messages.
   // This covers: git commit --no-edit, git commit --amend --no-edit
   // But excludes: git commit -m "...", git merge (auto-message)
   match self.source {
       Some(Message | Template | Merge | Squash) => return Ok(()),
       // ...
   }
   ```

### 7.4 README and Documentation Updates

**Actions:**

1. **Update README.md** to reflect any architectural changes

2. **Update CLAUDE.md** developer documentation with:
   - New module structure
   - Updated conventions
   - Refactoring decisions

3. **Create ARCHITECTURE.md** (new) explaining:
   - System overview with diagram
   - Module responsibilities
   - Data flow through the system
   - Extension points for new features

---

## 8. Testing Improvements

### 8.1 Test Organization

**Problem:** Limited test coverage for complex logic

**Current State:**

- Some unit tests in modules
- Integration tests in `tests/`
- No systematic test organization

**Actions:**

1. **Add module-specific test modules:**

   ```rust
   // At bottom of each module
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_basic_functionality() {
           // ...
       }

       #[tokio::test]
       async fn test_async_functionality() {
           // ...
       }
   }
   ```

2. **Priority areas needing tests:**
   - `diff/parser.rs` - Many edge cases in diff parsing
   - `generation/fallback.rs` - Fallback logic is critical
   - `config.rs` - Configuration loading and validation
   - `model.rs` - Token counting and truncation

3. **Add property-based tests for:**
   - Token counting (should never exceed limit after truncation)
   - Diff parsing (should handle any valid git diff)

### 8.2 Test Utilities

**Problem:** Testing requires complex setup

**Solution - Create Test Helpers:**

```rust
// In tests/common.rs

pub fn test_config() -> AppConfig {
    AppConfig {
        openai_api_key: Some("test-key".to_string()),
        model: Some("gpt-4o-mini".to_string()),
        max_tokens: Some(1024),
        max_commit_length: Some(72),
        timeout: Some(5),
    }
}

pub fn sample_diff() -> &'static str {
    include_str!("fixtures/sample.diff")
}

pub fn create_temp_repo() -> (TempDir, Repository) {
    // Helper to create a temporary git repo for testing
}

#[macro_export]
macro_rules! assert_valid_commit_message {
    ($msg:expr) => {
        assert!(!$msg.is_empty());
        assert!($msg.len() <= 72);
        assert!(!$msg.starts_with(' '));
        assert!(!$msg.ends_with(' '));
    };
}
```

### 8.3 Mock API Responses

**Problem:** Tests that require actual API calls are fragile

**Solution:**

```rust
// In tests/mocks.rs

pub struct MockOpenAIClient {
    responses: Vec<Result<String>>,
    call_count: AtomicUsize,
}

impl MockOpenAIClient {
    pub fn with_responses(responses: Vec<Result<String>>) -> Self {
        Self {
            responses,
            call_count: AtomicUsize::new(0),
        }
    }

    pub async fn generate(&self) -> Result<String> {
        let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
        self.responses[idx].clone()
    }
}
```

---

## Implementation Strategy

### Phase 1: Foundation (Week 1)

- [ ] Remove dead code markers and fix warnings
- [ ] Consolidate naming conventions (types and functions)
- [ ] Remove all commented code and resolve TODOs
- [ ] Centralize API key validation

### Phase 2: Structure (Week 2)

- [ ] Create new module structure (diff/, generation/, api/)
- [ ] Move code to new modules
- [ ] Update imports throughout
- [ ] Consolidate multi-step modules
- [ ] Run all tests and fix breakages

### Phase 3: Architecture (Week 3)

- [ ] Implement strategy pattern for fallbacks
- [ ] Refactor configuration management
- [ ] Consolidate error handling
- [ ] Update function signatures for consistency

### Phase 4: Optimization & Documentation (Week 4)

- [ ] Profile and optimize token counting
- [ ] Review and simplify parallel processing
- [ ] Add module-level documentation
- [ ] Add function documentation
- [ ] Create ARCHITECTURE.md

### Phase 5: Testing & Validation (Week 5)

- [ ] Add missing unit tests
- [ ] Create test utilities
- [ ] Add integration tests for new structure
- [ ] Performance regression testing
- [ ] Update README and documentation

---

## Validation Checklist

After completing refactoring, verify:

- [ ] `cargo clippy` produces zero warnings
- [ ] `cargo test` passes all tests
- [ ] `cargo build --release` succeeds
- [ ] Performance benchmarks show no regression
- [ ] All public APIs have documentation
- [ ] README accurately reflects new structure
- [ ] Integration tests pass in Docker
- [ ] Manual testing with actual git repos works

---

## Notes and Warnings

### Preserve Functionality

⚠️ **DO NOT** change behavior during refactoring. The goal is cleaner code with identical functionality.

### Test Extensively

⚠️ Run tests after every major change. Don't wait until the end.

### Incremental Changes

✅ Make small, focused commits. Don't try to refactor everything at once.

### Document Decisions

✅ Add comments explaining WHY code is structured a certain way, especially for non-obvious decisions.

### Consult Cursor Rules

✅ Follow the existing `.cursor/rules/` guidelines for Rust conventions and module structure.

---

## Expected Outcomes

After completing this refactoring:

1. **Code Organization**
   - Clear module boundaries with single responsibilities
   - 30% reduction in file sizes through better organization
   - Logical grouping of related functionality

2. **Maintainability**
   - New features easier to add (clear extension points)
   - Bugs easier to isolate and fix
   - Onboarding time for new contributors reduced

3. **Consistency**
   - Uniform naming conventions throughout
   - Consistent error handling patterns
   - Standardized documentation style

4. **Performance**
   - No performance regression
   - Potentially improved through simplification
   - Better visibility into performance characteristics

5. **Testing**
   - Higher test coverage
   - Easier to write new tests
   - More reliable test suite

---

## Questions and Decisions

If you encounter these situations during refactoring:

1. **"Should I keep this feature?"**
   - Check git history for usage
   - Look for references in issues/PRs
   - If unused and undocumented → remove

2. **"Which naming convention should I use?"**
   - Follow Rust API Guidelines
   - Be consistent with standard library
   - Prioritize clarity over brevity

3. **"Should I optimize this code?"**
   - Only if you have benchmarks showing it's slow
   - Don't optimize prematurely
   - Document performance characteristics

4. **"This could be done differently..."**
   - If purely stylistic → stick with refactoring goals
   - If architectural → note for future improvement
   - Don't scope creep beyond cleanup

---

## Additional Resources

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Effective Rust](https://www.lurklurk.org/effective-rust/)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)
- Project's `.cursor/rules/` for specific conventions

---

**End of Refactoring Guide**

_Last Updated: 2025-10-05_
_Version: 1.0_
