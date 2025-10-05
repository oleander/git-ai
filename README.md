# 🤖 Git AI

> **Intelligent commit messages with sophisticated multi-step analysis**

[![Rust](https://github.com/oleander/git-ai/actions/workflows/cd.yml/badge.svg)](https://github.com/oleander/git-ai/actions/workflows/cd.yml)
[![Crates.io](https://img.shields.io/crates/v/git-ai.svg)](https://crates.io/crates/git-ai)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Downloads](https://img.shields.io/crates/d/git-ai?style=flat-square)](https://crates.io/crates/git-ai)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange?style=flat-square)](https://www.rust-lang.org)

---

Git AI seamlessly integrates ChatGPT with git hooks to automate commit message generation based on your staged files. Using a **sophisticated multi-step analysis process**, it analyzes each file individually, calculates impact scores, and generates multiple commit message candidates before selecting the best one.

## ✨ Why Git AI?

🧠 **Multi-Step Analysis** - Sophisticated divide-and-conquer approach that analyzes files individually
⚡ **Lightning Fast** - Rust-powered with parallel processing and intelligent optimization
🎯 **Smart Integration** - Uses OpenAI's Assistant API, expertly tailored for git diffs
🧠 **Contextual Learning** - Maintains dedicated threads per project for improved relevance
🔄 **Intelligent Fallbacks** - Multiple fallback strategies ensure you always get meaningful messages
🏠 **Local Optimization** - Hosts exclusive assistant instance learning from all your projects

## 🚀 Quick Start

```bash
# Install Git AI
cargo install git-ai

# Set your OpenAI API key
git-ai config set openai-api-key sk-your-key-here

# Install the git hook in your repository
git-ai hook install

# Make changes, stage them, and commit without a message
git add .
git commit --all --no-edit
# ✨ Watch Git AI's multi-step analysis generate your perfect commit message!
```

## 🎬 How It Works

Git AI uses a sophisticated multi-step analysis process:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Git Commit  │────▶│ Parse Diff  │────▶│  Analyze    │────▶│   Score     │────▶│  Generate   │
│  (no msg)   │     │   Files     │     │   Files     │     │   Files     │     │  Messages   │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
                           │                    │                    │                    │
                           ▼                    ▼                    ▼                    ▼
                    ┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
                    │ auth.rs     │     │ Lines: +50  │     │ Score: 0.95 │     │ Candidates: │
                    │ test.rs     │     │ Lines: -10  │     │ Score: 0.65 │     │ 1. "Add JWT"│
                    │ main.rs     │     │ Category:   │     │ Score: 0.62 │     │ 2. "auth:   │
                    └─────────────┘     │   source    │     └─────────────┘     │    impl"    │
                                       └─────────────┘                           └─────────────┘
                                                                                         │
                                                                                         ▼
                                                                                 ┌─────────────┐
                                                                                 │   Select    │
                                                                                 │    Best     │
                                                                                 │  Message    │
                                                                                 └─────────────┘
```

### Multi-Step Process

1. **Parse** - Splits the git diff into individual files
   - Handles different diff formats (standard, commit with hash, raw output)
   - Extracts file paths, operation types, and diff content
   - Supports added, modified, deleted, renamed, and binary files

2. **Analyze** - Examines each file in parallel for:
   - Lines added/removed (counts actual +/- lines)
   - File type categorization (source, test, config, docs, binary, build)
   - Change significance and summary generation
   - Uses OpenAI function calling for structured analysis

3. **Score** - Calculates impact scores based on:
   - Operation type weights (add: 0.3, modify: 0.2, delete: 0.25, rename: 0.1, binary: 0.05)
   - File category weights (source: 0.4, test: 0.2, config: 0.25, build: 0.3, docs: 0.1, binary: 0.05)
   - Lines changed (normalized up to 0.3)
   - Total score capped at 1.0

4. **Generate** - Creates multiple commit message candidates
   - Action-focused style (e.g., "Add authentication")
   - Component-focused style (e.g., "auth: implementation")
   - Impact-focused style (e.g., "New feature for authentication")
   - Respects max length constraints

5. **Select** - Chooses the best message based on:
   - Highest impact files
   - Overall change context
   - Conventional commit format when appropriate

### Intelligent Fallback Strategy

```
┌──────────────────┐
│ Multi-Step + API │ ──── Fail ───┐
└────────┬─────────┘              │
         │ Success                ▼
         ▼                 ┌──────────────────┐
   ┌───────────┐           │ Local Multi-Step │ ──── Fail ───┐
   │  Message  │           └────────┬─────────┘              │
   │ Generated │ ◀──────────────────┘ Success                ▼
   └───────────┘                                   ┌──────────────────┐
         ▲                                         │ Single-Step API  │
         └─────────────────────────────────────────┴──────────────────┘
```

Git AI automatically falls back through multiple strategies:

1. **Multi-Step with API** - Full analysis using OpenAI's function calling
2. **Local Multi-Step** - Local analysis without API (when API is unavailable)
3. **Single-Step API** - Direct prompt-based generation as final fallback

This ensures you always get meaningful commit messages, even when the API is unavailable.

## 🌟 Key Features

### 🧠 **Multi-Step Analysis (Default)**

Uses a sophisticated divide-and-conquer approach that analyzes each file individually, calculates impact scores, and generates multiple commit message candidates before selecting the best one.

### 🎯 **Smart Integration**

Leverages OpenAI's powerful Assistant API, expertly tailored to transform git diffs into insightful commit messages.

### 📚 **Contextual Learning**

Maintains a dedicated thread for each project, allowing the assistant to build context over time and improve performance and message relevance with every commit.

### 🏠 **Local Optimization**

Hosts an exclusive assistant instance on your machine, learning from all your projects to elevate the quality of commit messages throughout your development environment.

### 🛡️ **Intelligent Fallbacks**

Automatically falls back to local analysis when API is unavailable, ensuring you always get meaningful commit messages.

## 📦 Installation

### Option 1: Cargo (Recommended)

```bash
cargo install git-ai
```

### Option 2: Pre-compiled Binaries

```bash
cargo install cargo-binstall
cargo binstall git-ai
```

### Option 3: From Source

```bash
git clone https://github.com/oleander/git-ai
cd git-ai
cargo install --path .
```

### Prerequisites

- Rust and Cargo installed on your machine
- Git repository
- OpenAI API key

## 🎯 Usage

### Basic Setup

```bash
# 1. Configure your API key
git-ai config set openai-api-key sk-your-key-here

# 2. Install the git hook
git-ai hook install

# 3. Start committing with multi-step analysis!
git add some-file.rs
git commit --all --no-edit  # Git AI takes over here
```

### Advanced Configuration

```bash
# Choose your AI model
git-ai config set model gpt-4.1       # Latest (default)
git-ai config set model gpt-4o        # Optimized, better quality
git-ai config set model gpt-4o-mini   # Faster processing
git-ai config set model gpt-4         # Original GPT-4

# Customize output and performance
git-ai config set max-commit-length 72    # Limit message length
git-ai config set max-tokens 512          # Control API usage (default)

# Reset to defaults
git-ai config reset
```

### Hook Management

```bash
git-ai hook install      # Install hook in current repo
git-ai hook uninstall    # Remove hook
git-ai hook reinstall    # Reinstall hook
```

## 🛠️ Development

### Using Justfile Commands

The project includes a Justfile with useful development commands:

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

### Building from Source

```bash
# Build the project
cargo build

# Run tests
cargo test

# Install locally for development
cargo install --path .

# Quick local installation with hook setup
just local-install
```

## 📖 Examples

### Feature Addition with Multi-Step Analysis

```diff
// auth.rs (Score: 0.95 - High impact source file)
+ fn validate_jwt_token(token: &str) -> Result<Claims, AuthError> {
+     decode::<Claims>(token, &DecodingKey::from_secret("secret"), &Validation::default())
+ }

// test.rs (Score: 0.65 - Supporting test file)
+ #[test]
+ fn test_jwt_validation() {
+     assert!(validate_jwt_token("valid_token").is_ok());
+ }
```

**Generated commit:** `Add JWT token validation with comprehensive error handling`

### Bug Fix Analysis

```diff
// config.rs (Score: 0.82 - Important config change)
- if user.age > 18 {
+ if user.age >= 18 {
```

**Generated commit:** `Correct age validation to include 18-year-olds in config`

## ⚙️ Configuration Reference

| Setting             | Description                | Default   |
| ------------------- | -------------------------- | --------- |
| `openai-api-key`    | Your OpenAI API key        | Required  |
| `model`             | AI model to use            | `gpt-4.1` |
| `max-tokens`        | Maximum tokens per request | `512`     |
| `max-commit-length` | Max commit message length  | `72`      |

## 🏗️ Architecture

### Core Components

- **CLI Interface** (`src/main.rs`) - Command-line interaction and configuration
- **Git Hook** (`src/bin/hook.rs`) - Prepare-commit-msg hook integration
- **Multi-Step Analysis** (`src/multi_step_analysis.rs`, `src/multi_step_integration.rs`) - Sophisticated file analysis and scoring
- **Diff Processing** (`src/hook.rs`) - Parallel processing and optimization
- **API Integration** (`src/openai.rs`, `src/ollama.rs`) - OpenAI and Ollama support
- **Function Calling** (`src/function_calling.rs`) - Structured commit message generation

### Key Workflows

1. **Hook Installation** - Symlinks executable to `.git/hooks/prepare-commit-msg`
2. **Multi-Step Analysis** - Parse → Analyze → Score → Generate → Select
3. **Intelligent Fallbacks** - API → Local → Single-step as needed
4. **Performance Optimization** - Parallel processing, token management, smart truncation

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run integration tests
./scripts/integration-tests

# Test hook functionality
./scripts/hook-stress-test

# Run comprehensive test suite
./scripts/comprehensive-tests
```

## 🚀 Roadmap

- [ ] 🌐 Support for more AI providers (Anthropic, Cohere)
- [ ] 🎨 Customizable commit message templates
- [ ] 📊 Enhanced contextual learning across projects
- [ ] 🔄 Integration with popular Git GUIs
- [ ] 🌍 Multi-language commit message support

## ❓ FAQ

**Q: How does multi-step analysis improve commit messages?**
A: By analyzing files individually and calculating impact scores, Git AI understands which changes are most significant and crafts messages that reflect the true purpose of your commit.

**Q: What happens if the API is down?**
A: Git AI automatically falls back to local multi-step analysis, then single-step API if needed. You'll always get a meaningful commit message.

**Q: Will this work with any Git repository?**
A: Yes! Git AI works with any Git repository. Just install the hook and you're ready to go.

**Q: What if I want to write my own commit message?**
A: Just use `git commit -m "your message"` as usual. Git AI only activates when no message is provided.

## 🤝 Contributing

Your feedback and contributions are welcome! Join our community to help improve Git AI by submitting issues, offering suggestions, or contributing code. See our [contributing guidelines](CONTRIBUTING.md) for more details.

## 📜 License

Git AI is proudly open-sourced under the MIT License. See [LICENSE](LICENSE) for more details.

---

**Made with ❤️ by developers, for developers**

[⭐ Star this repo](https://github.com/oleander/git-ai) if Git AI's multi-step analysis improves your Git workflow!
