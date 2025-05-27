# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Git AI is a Rust-based CLI tool that seamlessly integrates with git hooks to automate commit message generation based on staged changes. It leverages OpenAI's API to analyze git diffs and create contextually relevant commit messages.

## Development Commands

### Building and Installation

```bash
# Build the project
cargo build

# Run tests
cargo test

# Install locally from source (for development)
cargo install --path .

# Install the git hook in the current repository
git-ai hook install
```

### Configuration

```bash
# Set OpenAI API key
git-ai config set openai-api-key <your-key>

# Set the model to use
git-ai config set model gpt-4.1     # Default model (latest version)
git-ai config set model gpt-4o      # Optimized GPT-4, better quality but slower
git-ai config set model gpt-4o-mini # Mini version, faster processing
git-ai config set model gpt-4       # Original GPT-4

# Set max tokens for API requests
git-ai config set max-tokens <number>

# Set maximum commit message length
git-ai config set max-commit-length <number>

# Reset configuration to defaults
git-ai config reset
```

### Hook Management

```bash
# Install the git hook
git-ai hook install

# Uninstall the git hook
git-ai hook uninstall

# Reinstall the git hook
git-ai hook reinstall
```

### Alternative Installation

```bash
# Install precompiled binary with cargo-binstall
cargo install cargo-binstall
cargo binstall git-ai
```

## Architecture

The project is structured into several core components:

1. **CLI Interface** (`src/main.rs`): Provides the command-line interface for configuring the tool and managing hooks.

2. **Git Hook** (`src/bin/hook.rs`): The actual Git hook that's invoked during the commit process to generate commit messages.

3. **Diff Processing** (`src/hook.rs`): Handles parsing and processing Git diffs, with performance optimizations for handling large diffs through parallel processing and token management.

4. **API Integration**:

   - **OpenAI** (`src/openai.rs`): Manages communication with the OpenAI API, handling request creation, error handling, and response parsing.
   - **Ollama** (`src/ollama.rs`): Provides integration with local Ollama models as an alternative to OpenAI.

5. **Model Management** (`src/model.rs`): Defines supported AI models (GPT-4, GPT-4o, GPT-4o-mini, GPT-4.1) and handles token counting/management with optimization strategies for different text lengths.

6. **Configuration** (`src/config.rs`): Manages user configuration including API keys, model preferences, and token limits.

7. **Commit Generation** (`src/commit.rs`): Coordinates the process of generating commit messages from diffs.

8. **Function Calling** (`src/function_calling.rs`): Implements OpenAI function calling for structured commit message generation with reasoning and file change summaries.

9. **Profiling** (`src/profiling.rs`): Performance profiling utilities to measure execution time of various operations.

## Key Workflows

1. **Hook Installation**: When `git-ai hook install` is run, the tool symlinks its executable to the repository's `.git/hooks/prepare-commit-msg` hook.

2. **Commit Message Generation**:

   - When a user runs `git commit` without specifying a message
   - The hook intercepts the commit process
   - Retrieves the staged changes (diff)
   - Processes the diff to fit within token limits
   - Sends the processed diff to OpenAI
   - Uses the AI response as the commit message

3. **Performance Optimization**:
   - Parallel processing for large diffs
   - Token management to ensure API limits aren't exceeded
   - String pooling to reduce memory allocations
   - Smart truncation to prioritize more relevant parts of large diffs
   - Tiered token counting approaches based on text length for better performance

## Testing

The project uses integration tests to verify core functionality:

```bash
# Run all tests
cargo test

# Run specific tests
cargo test test_empty_diff

# Run comprehensive tests
./scripts/comprehensive-tests

# Run integration tests
./scripts/integration-tests

# Test hook functionality
./scripts/hook-stress-test
```

Test files are located in the `tests/` directory and include utilities for creating test repositories and verifying diff operations.
