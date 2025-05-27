# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Git AI is a Rust-based CLI tool that seamlessly integrates with git hooks to automate commit message generation based on staged changes. It leverages OpenAI's API to analyze git diffs and create contextually relevant commit messages. The tool uses a sophisticated multi-step analysis process to generate meaningful commit messages by analyzing individual files, calculating impact scores, and selecting the best message from multiple candidates.

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

# Quick local installation for development with hook setup
just local-install
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

### Using the Justfile

The project includes a Justfile with useful commands:

```bash
# Install locally with debug symbols and setup hooks
just local-install

# Run integration tests in Docker
just integration-test

# Build Docker image
just docker-build

# Run GitHub Actions locally
just local-github-actions
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

9. **Multi-Step Analysis** (`src/multi_step_analysis.rs` and `src/multi_step_integration.rs`): Implements the sophisticated divide-and-conquer approach that analyzes files individually, calculates impact scores, and generates multiple commit message candidates.

10. **Profiling** (`src/profiling.rs`): Performance profiling utilities to measure execution time of various operations.

## Key Workflows

1. **Hook Installation**: When `git-ai hook install` is run, the tool symlinks its executable to the repository's `.git/hooks/prepare-commit-msg` hook.

2. **Commit Message Generation**:

   - When a user runs `git commit` without specifying a message
   - The hook intercepts the commit process
   - Retrieves the staged changes (diff)
   - Processes the diff to fit within token limits
   - Sends the processed diff to OpenAI
   - Uses the AI response as the commit message

3. **Multi-Step Analysis Process**:

   - **Parse**: Splits the git diff into individual files
   - **Analyze**: Examines each file for lines added/removed, file type, and change significance
   - **Score**: Calculates impact scores based on operation type, file category, and lines changed
   - **Generate**: Creates multiple commit message candidates
   - **Select**: Chooses the best message based on highest impact

4. **Performance Optimization**:

   - Parallel processing for large diffs
   - Token management to ensure API limits aren't exceeded
   - String pooling to reduce memory allocations
   - Smart truncation to prioritize more relevant parts of large diffs
   - Tiered token counting approaches based on text length for better performance

5. **Intelligent Fallback Strategy**:
   - First attempts multi-step analysis with API
   - Falls back to local multi-step if API fails
   - Falls back to single-step API as a last resort

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

## Example Usage

After installation and configuration:

```bash
# Make changes to your code
# Stage changes
git add .

# Commit without a message - Git AI will generate one
git commit --all --no-edit
```

The commit message will be automatically generated based on the staged changes.
