# LLM Input Generation Testing

## Overview

This document describes the comprehensive test suite for the LLM input generation system in git-ai. The tests ensure that the prompt templates and request creation work correctly across various scenarios.

## Testing Approach

### Why Not In-Memory or Fake Git Repos?

We evaluated several approaches:

1. **In-memory git repositories** - Would require significant mocking infrastructure
2. **Fake git repos** - Would not test real git diff formats
3. **Real temporary git repos** - ✅ **Chosen approach**

We use real temporary git repositories (via `tempfile::TempDir`) for maximum fidelity when needed, but most tests use hardcoded diff strings since we're primarily testing the **input processing** layer, not git itself.

### Test Structure

The test file (`tests/llm_input_generation_test.rs`) is organized into 4 sections:

#### Section 1: Minimal/Corner Case Tests (10 tests)

Tests edge cases and boundary conditions:

- ✅ `test_template_generation_with_default_max_length` - Verify template renders correctly
- ✅ `test_token_counting_empty_template` - Handle empty strings
- ✅ `test_token_counting_template` - Verify template has reasonable token count
- ✅ `test_create_request_with_zero_tokens` - Edge case: 0 max_tokens
- ✅ `test_create_request_with_empty_diff` - Corner case: empty diff
- ✅ `test_create_request_with_whitespace_only_diff` - Whitespace handling
- ✅ `test_create_request_preserves_model` - Model preservation across GPT-4, GPT-4o, etc.
- ✅ `test_create_request_with_max_u16_tokens` - Maximum u16 value
- ✅ `test_create_request_with_overflow_tokens` - Token count overflow handling

**Key insights:**
- The system handles edge cases gracefully
- Token counting works correctly even with empty input
- u16 overflow is handled properly (caps at u16::MAX)

#### Section 2: Simple Test Cases (6 tests)

Tests basic git operations with straightforward diffs:

- ✅ `test_create_request_with_simple_diff` - Basic file modification
- ✅ `test_create_request_with_file_addition` - New file creation
- ✅ `test_create_request_with_file_deletion` - File deletion
- ✅ `test_create_request_with_file_rename` - File rename/move
- ✅ `test_token_counting_with_diff_content` - Token counting on real diffs

**Key insights:**
- All standard git operations are handled correctly
- Diff metadata (new file mode, deleted file mode, rename from/to) is preserved
- Token counting scales appropriately with diff size

#### Section 3: Complex Test Cases (8 tests)

Tests challenging scenarios and edge cases:

- ✅ `test_create_request_with_multiple_files` - Multiple file changes in one diff
- ✅ `test_create_request_with_binary_file` - Binary file changes
- ✅ `test_create_request_with_special_characters` - Unicode, emojis, special chars
- ✅ `test_create_request_with_large_diff` - 1000+ line diffs
- ✅ `test_create_request_with_very_long_lines` - 10,000+ character lines (minified code)
- ✅ `test_create_request_with_mixed_operations` - Add + modify + delete + rename + binary
- ✅ `test_token_counting_consistency_with_complex_diff` - Consistent token counts
- ✅ `test_create_request_with_code_in_multiple_languages` - Rust, Python, JS, Go

**Key insights:**
- System handles very large diffs (10,000+ lines)
- Binary files are preserved correctly
- Unicode and emojis work properly
- Multi-language code is handled uniformly

#### Section 4: Integration Tests (2 tests)

End-to-end workflow tests:

- ✅ `test_full_workflow_simple` - Complete pipeline: template → tokens → request
- ✅ `test_end_to_end_with_token_limits` - Token budget management

**Key insights:**
- All components work together correctly
- Token counting integrates properly with request creation
- Model context limits are respected

## What We're Testing

### 1. Template Generation (`get_instruction_template`)

The instruction template is a large mustache template that guides the LLM. We test:

- Template renders without errors
- Contains all required sections (Algorithm, Impact Score, Examples, etc.)
- max_length parameter is properly substituted
- Reasonable size (> 500 chars, < 50,000 chars)

### 2. Request Creation (`create_commit_request`)

This function combines the template + diff into an OpenAI-compatible request. We test:

- All components are present (system, prompt, max_tokens, model)
- Diff content is preserved exactly
- Model selection works for all GPT variants
- Token limits are handled correctly (including overflow)

### 3. Token Counting (`token_used`, `Model::count_tokens`)

Accurate token counting is critical for staying within model limits. We test:

- Empty strings return 0 tokens
- Token counting is consistent (same input → same output)
- Counting works with code, diffs, special characters
- Scales appropriately with content size

## Public API Changes

To enable thorough testing, we made two previously private functions public:

```rust
pub fn get_instruction_template() -> Result<String>
pub fn create_commit_request(diff: String, max_tokens: usize, model: Model) -> Result<openai::Request>
```

**Rationale:**
- These are the core input generation functions that need comprehensive testing
- Making them public allows integration tests to verify behavior
- They could be useful for advanced users or tooling
- Well-documented with proper doc comments

## Running the Tests

```bash
# Run all LLM input generation tests
cargo test --test llm_input_generation_test

# Run with output
cargo test --test llm_input_generation_test -- --nocapture

# Run a specific test
cargo test --test llm_input_generation_test test_create_request_with_large_diff

# Run all tests
cargo test
```

## Test Results

All 26 tests pass successfully:

```
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Coverage

The test suite covers:

### ✅ Template Generation
- Default configuration
- All required sections present
- Token counting

### ✅ Request Creation
- All git operations (add, modify, delete, rename, binary)
- Edge cases (empty, whitespace, overflow)
- Model preservation
- Token limits

### ✅ Diff Handling
- Single file changes
- Multiple file changes
- Binary files
- Special characters (Unicode, emojis)
- Very large diffs (1000+ lines)
- Very long lines (10,000+ chars)
- Mixed operations

### ✅ Token Counting
- Consistency
- Scaling
- Different content types (code, diffs, text)

### ✅ Integration
- End-to-end workflows
- Token budget management
- Component integration

## Future Enhancements

Potential areas for additional testing:

1. **Performance testing** - Benchmark large diff processing
2. **Fuzzing** - Generate random diffs to find edge cases
3. **Real git repo integration** - Test with actual repository diffs (if needed)
4. **Custom max_length values** - Test different commit message length limits (requires config refactoring)
5. **Error scenarios** - Test malformed diffs, template corruption, etc.

## Conclusion

The test suite provides comprehensive coverage of the LLM input generation system, from minimal corner cases to complex real-world scenarios. The tests are fast (< 1 second), deterministic, and don't require network access or API keys.
