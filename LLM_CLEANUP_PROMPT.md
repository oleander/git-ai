# LLM Prompt: Git AI Codebase Cleanup and Refactoring

## Context

You are tasked with refactoring the Git AI codebase - a Rust CLI tool that generates intelligent git commit messages using OpenAI's API. The codebase has grown organically and needs systematic cleanup to improve maintainability.

## Project Overview

- **Language:** Rust 2021 edition
- **Purpose:** Automated commit message generation using AI
- **Architecture:** Git hook → Diff parsing → Multi-step AI analysis → Message generation
- **Key Features:** Multi-step analysis, intelligent fallbacks, parallel processing

## Core Principles

1. **Preserve Functionality:** DO NOT change behavior, only improve structure
2. **Incremental Changes:** Make small, focused commits
3. **Test Extensively:** Run `cargo test` after each major change
4. **Document Decisions:** Explain WHY, not just WHAT

---

## Priority Issues to Address

### 1. CRITICAL: Remove Dead Code Markers

**Files:** `src/hook.rs`, `src/bin/hook.rs`

**Actions:**

1. Remove `#![allow(dead_code)]` from top of both files
2. Run `cargo clippy` and fix ALL dead code warnings
3. Specifically investigate and remove if unused:
   - `StringPool` struct in hook.rs (lines 62-85)
   - Duplicate `Args` struct in hook.rs
4. For legitimately unused public API, add doc comments explaining purpose

**Validation:** `cargo clippy` should produce ZERO warnings

---

### 2. HIGH: Naming Consistency

**Type Naming:**

```rust
// BEFORE (inconsistent)
App / Settings (same concept, different names)
FileAnalysisResult / FileWithScore / FileDataForScoring (similar but different)
Response (in openai.rs) vs CommitFunctionArgs (in function_calling.rs)

// AFTER (consistent)
AppConfig (rename from App, remove Settings alias)
FileChange (unified type with optional analysis fields)
CommitResponse (unified response type)
```

**Function Naming - Apply These Prefixes Consistently:**

- `generate_*` - producing text/messages
- `create_*` - building objects
- `parse_*` - converting formats
- `analyze_*` - examining data
- `calculate_*` - computing values
- `get_*` / `fetch_*` - retrieving data

**Examples:**

```rust
// Fix these:
get_instruction_template() → generate_instruction_template()
token_used() → calculate_token_usage()
call_analyze_function() → analyze_file_via_api()
```

---

### 3. HIGH: Module Restructuring

**Current Problem:** Large files mixing concerns (hook.rs is 725 lines)

**Target Structure:**

```
src/
├── diff/           # NEW - Extract from hook.rs
│   ├── mod.rs      # Public API and re-exports
│   ├── parser.rs   # Git diff parsing (hook.rs lines 212-402)
│   ├── processor.rs # PatchDiff trait implementation
│   ├── optimization.rs # Parallel processing (process_chunk)
│   └── traits.rs   # FilePath, Utf8String, DiffDeltaPath
│
├── generation/     # NEW - Consolidate multi_step_* modules
│   ├── mod.rs
│   ├── multi_step.rs  # Merge: multi_step_analysis + multi_step_integration
│   ├── fallback.rs    # Centralized fallback logic + simple_multi_step
│   ├── local.rs       # Local generation without API
│   └── api.rs         # API-based generation
│
├── api/            # NEW - Organize external integrations
│   ├── mod.rs
│   ├── openai.rs   # Current openai.rs refactored
│   ├── ollama.rs   # Keep as-is
│   └── auth.rs     # NEW - API key validation
│
└── core/           # Existing core functionality
    ├── config.rs
    ├── model.rs
    ├── filesystem.rs
    └── profiling.rs
```

**Implementation Order:**

1. Create new module directories and mod.rs files
2. Move code to new locations (keep git history with `git mv` where possible)
3. Update all imports
4. Run tests after each module move

---

### 4. MEDIUM: Consolidate Fallback Logic

**Current Problem:** Fallback scattered across commit.rs, openai.rs, multi_step_integration.rs

**Solution - Implement Strategy Pattern in generation/fallback.rs:**

```rust
pub trait GenerationStrategy: Send + Sync {
    async fn generate(&self, diff: &str, config: &AppConfig) -> Result<String>;
    fn name(&self) -> &str;
}

pub struct MultiStepAPIStrategy { /* OpenAI multi-step */ }
pub struct LocalMultiStepStrategy { /* Local analysis */ }
pub struct SingleStepAPIStrategy { /* Simple API call */ }

pub async fn generate_with_fallback(
    diff: &str,
    config: &AppConfig,
) -> Result<String> {
    let strategies: Vec<Box<dyn GenerationStrategy>> = vec![
        Box::new(MultiStepAPIStrategy::new(config)?),
        Box::new(LocalMultiStepStrategy),
        Box::new(SingleStepAPIStrategy::new(config)?),
    ];

    for strategy in strategies {
        match strategy.generate(diff, config).await {
            Ok(msg) => return Ok(msg),
            Err(e) if e.to_string().contains("invalid_api_key") => return Err(e),
            Err(e) => log::warn!("{} failed: {}", strategy.name(), e),
        }
    }

    bail!("All generation strategies failed")
}
```

---

### 5. MEDIUM: API Key Validation

**Current Problem:** Duplicated in commit.rs and openai.rs

**Solution - Create api/auth.rs:**

```rust
/// Validates and retrieves API key from config or environment
pub fn get_api_key(config: &AppConfig) -> Result<String> {
    // Try config first
    if let Some(key) = &config.openai_api_key {
        return validate_api_key(key);
    }

    // Try environment
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        return validate_api_key(&key);
    }

    bail!(
        "OpenAI API key not found. Set via:\n\
         1. git-ai config set openai-api-key <key>\n\
         2. OPENAI_API_KEY environment variable"
    )
}

fn validate_api_key(key: &str) -> Result<String> {
    if key.is_empty() || key == "<PLACE HOLDER FOR YOUR API KEY>" {
        bail!("Invalid or placeholder API key")
    }
    Ok(key.to_string())
}
```

**Usage:** Replace all API key validation code with `api::auth::get_api_key(&config)?`

---

### 6. MEDIUM: Remove Technical Debt

**TODO Comments:**

```bash
# Find all TODOs
grep -r "TODO" src/ --include="*.rs"

# For each TODO:
# 1. If trivial → implement immediately
# 2. If complex → create GitHub issue, reference in comment
# 3. If obsolete → remove
```

**Specific TODOs in model.rs:**

- Line 14: Remove commented import or use it
- Line 25: Move DEFAULT_MODEL to shared constants
- Line 214: Implement model-specific tokenizer selection

**Commented Code:**

- Remove ALL commented-out code unless it has explicit reason with issue reference
- Exception: Keep if labeled with removal date and issue number

**Duplicate Logic:**
Consolidate these duplicates:

- Diff file extraction: openai.rs, multi_step_integration.rs, simple_multi_step.rs → diff/parser.rs
- API key validation: commit.rs, openai.rs → api/auth.rs
- Commit message generation: scattered → generation/ module

---

### 7. LOW: Documentation

**Add to EVERY Module (module-level docs):**

````rust
//! Brief purpose of module.
//!
//! # Overview
//! Detailed explanation of what and why.
//!
//! # Examples
//! ```rust,no_run
//! use git_ai::module::function;
//! let result = function()?;
//! ```
````

**Add to EVERY Public Function:**

````rust
/// Brief one-line description.
///
/// Detailed explanation if needed.
///
/// # Arguments
/// * `param` - Description
///
/// # Returns
/// Description of return value
///
/// # Errors
/// When this function returns errors
///
/// # Examples
/// ```rust,no_run
/// let result = function(arg)?;
/// ```
````

**Priority Modules Needing Docs:**

- hook.rs → explain diff processing architecture
- multi*step*\*.rs → explain analysis algorithm
- model.rs → explain token management strategy

---

## Implementation Phases

### Phase 1: Quick Wins (Start Here)

1. Remove `#![allow(dead_code)]` and fix warnings
2. Remove all commented code
3. Resolve simple TODO comments
4. Run `cargo fix --allow-dirty --allow-staged`
5. Run `cargo clippy --fix --allow-dirty --allow-staged`

**Validation:** Zero clippy warnings

### Phase 2: Naming Consistency

1. Rename `App` → `AppConfig` throughout
2. Consolidate file-related types to single `FileChange` struct
3. Unify response types to `CommitResponse`
4. Apply consistent verb prefixes to functions
5. Update imports and fix compilation

**Validation:** `cargo test` passes

### Phase 3: Module Restructuring

1. Create new module structure (diff/, generation/, api/)
2. Move code incrementally, testing after each move
3. Update imports throughout codebase
4. Consolidate multi*step*\* modules into generation/multi_step.rs

**Validation:** `cargo test` passes, file count reduced

### Phase 4: Architecture Improvements

1. Implement strategy pattern for fallbacks
2. Centralize API key validation
3. Remove duplicate logic
4. Improve error handling consistency

**Validation:** `cargo test` passes, integration tests pass

### Phase 5: Documentation & Polish

1. Add module-level documentation
2. Document all public functions
3. Add inline comments for complex logic
4. Update README.md if structure changed

**Validation:** `cargo doc` builds without warnings

---

## Testing Requirements

**After EVERY Change:**

```bash
cargo build
cargo test
cargo clippy
```

**Before Committing:**

```bash
cargo test --release
./scripts/integration-tests  # If available
cargo build --release
```

**Test Priority Areas:**

- Diff parsing (many edge cases)
- Fallback logic (critical path)
- Configuration loading
- Token counting and truncation

---

## Common Pitfalls to Avoid

❌ **Don't:** Change behavior while refactoring
✅ **Do:** Refactor first, improve behavior in separate commits

❌ **Don't:** Make massive changes in one commit
✅ **Do:** Small, focused commits with clear messages

❌ **Don't:** Optimize without profiling
✅ **Do:** Document current behavior, optimize later if needed

❌ **Don't:** Remove public API without checking usage
✅ **Do:** Mark as deprecated first, remove in next major version

❌ **Don't:** Skip tests "to save time"
✅ **Do:** Test continuously, catch issues early

---

## File-Specific Instructions

### src/hook.rs

- **Size:** 725 lines (too large)
- **Extract:** Lines 212-402 → diff/parser.rs
- **Extract:** process_chunk function → diff/optimization.rs
- **Extract:** Utility traits → diff/traits.rs
- **Remove:** StringPool if unused, or implement if intended
- **Remove:** `#![allow(dead_code)]` directive

### src/multi_step_analysis.rs

- **Merge with:** multi_step_integration.rs into generation/multi_step.rs
- **Organize into:** Submodules (analysis, scoring, candidates, selection)
- **Keep:** All function signatures initially (maintain API compatibility)

### src/multi_step_integration.rs

- **Merge with:** multi_step_analysis.rs
- **Extract:** parse_diff function → diff/parser.rs (consolidate with others)
- **Refactor:** Parallel analysis to use clearer async patterns

### src/simple_multi_step.rs

- **Merge into:** generation/fallback.rs
- **Purpose:** Serves as fallback strategy, fits naturally there

### src/openai.rs

- **Move to:** api/openai.rs
- **Extract:** API key validation → api/auth.rs
- **Extract:** Fallback logic → generation/fallback.rs
- **Simplify:** Remove duplicate generation functions

### src/config.rs

- **Rename:** `App` → `AppConfig`
- **Remove:** `Settings` type alias
- **Keep:** Lazy static for convenience, but add non-static constructor
- **Document:** All configuration options

### src/model.rs

- **Resolve:** All TODO comments
- **Simplify:** Token truncation logic (remove walk_truncate, keep single method)
- **Document:** Performance characteristics of token counting

---

## Code Quality Checklist

Before marking refactoring complete, verify:

- [ ] `cargo clippy` produces **zero warnings**
- [ ] `cargo test` passes **all tests**
- [ ] `cargo build --release` **succeeds**
- [ ] All public functions **have documentation**
- [ ] All modules **have module-level docs**
- [ ] No files **exceed 500 lines**
- [ ] No functions **exceed 100 lines**
- [ ] Naming is **consistent** throughout
- [ ] No duplicate **logic** (DRY principle)
- [ ] Error messages are **helpful and actionable**
- [ ] Git history is **clean** with focused commits

---

## Expected Outcomes

**Quantitative:**

- 30% reduction in average file size
- 50% reduction in code duplication
- 100% of public API documented
- Zero clippy warnings
- Improved test coverage

**Qualitative:**

- Clearer module boundaries
- Easier to onboard new contributors
- Simpler to add new features
- Better error messages
- More maintainable codebase

---

## Emergency Rollback

If refactoring introduces bugs:

1. **Don't panic** - use git to recover
2. **Identify** the breaking commit with `git bisect`
3. **Revert** specific commits: `git revert <commit-hash>`
4. **Extract** working parts, discard broken changes
5. **Re-test** thoroughly before continuing

Always commit working states frequently!

---

## Final Notes

**Remember:**

- Refactoring is an investment in future productivity
- Perfect is the enemy of good - aim for significant improvement, not perfection
- When in doubt, consult `.cursor/rules/` for project-specific conventions
- Document your decisions so future developers understand the "why"

**If you encounter ambiguity:**

- Prefer clarity over cleverness
- Follow Rust API guidelines
- Be consistent with standard library patterns
- Ask for clarification rather than guessing

---

**Start with Phase 1 (Quick Wins) and work incrementally through each phase.**

Good luck! 🦀
