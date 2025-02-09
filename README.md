# Git AI [![Rust](https://github.com/oleander/git-ai/actions/workflows/cd.yml/badge.svg)](https://github.com/oleander/git-ai/actions/workflows/cd.yml) [![Crates.io](https://img.shields.io/crates/v/git-ai.svg)](https://crates.io/crates/git-ai) [![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Git AI** seamlessly integrates ChatGPT with git hooks to automate commit message generation based on your staged files. Stage your changes and commit without a message; **Git AI** does the rest, crafting detailed commit messages that reflect the essence of your changes.

### Key Features

- **Smart Integration**: Leverages OpenAI's powerful Assistant API, expertly tailored to transform git diffs into insightful commit messages.
- **Contextual Learning**: This feature maintains a dedicated thread for each project, allowing the assistant to build context over time and thereby improving performance and message relevance with every commit.
- **Local Optimization**: Hosts an exclusive assistant instance on your machine, learning from all your projects to elevate the quality of commit messages throughout your development environment.

## Quick Start

```bash
# Install Git AI from source
cargo install git-ai

# alternatively, install a precompiled binary with cargo-binstall
cargo install cargo-binstall

# install the precompiled binary
cargo binstall git-ai

# Set your OpenAI API key
git-ai config set openai-api-key <your key>

# Install the Git AI hook in your repo
git-ai hook install

# Make your changes, stage them, and commit without a message
git commit --all --no-edit
```

## Getting Started

### Prerequisites

- Rust and Cargo installed on your machine.

### Installation Options

#### Via Crates.io

```bash
cargo install git-ai
git-ai config set openai-api-key <api-key>
git-ai hook install
```

#### From Source

```bash
git clone https://github.com/oleander/git-ai
cd git-ai
cargo install --path .
```

## Usage Guide

### Setting Up

- Follow the installation instructions to get Git AI ready.
- Initialize Git AI in your repository with `git-ai hook install`.
- Set your OpenAI API key using `git-ai config set openai-api-key <api-key>`.

### Advanced Configuration

Customize Git AI's behavior with these commands:

- `git-ai config set max-commit-length <length>` (default: 72): Set the maximum length of commit messages.
- `git-ai config set max-tokens <tokens>` (default: 512): Set the maximum number of tokens for the assistant.
- `git-ai config set model <model>` (default: "gpt-3.5-turbo"): Set the OpenAI model to use.
- `git-ai config set openai-api-key <api-key>`: Set your OpenAI API key.
- `git-ai config set url <url>` (default: "https://api.openai.com/v1"): Set the OpenAI API URL. Useful for using alternative OpenAI-compatible APIs or proxies.

## Contributing

Your feedback and contributions are welcome! Join our community to help improve **Git AI**by submitting issues, offering suggestions, or contributing code. See our [contributing guidelines](CONTRIBUTING.md) for more details.

## Testing

Run `cargo test` to execute the test suite and ensure everything functions as expected.

## License

**Git AI** is proudly open-sourced under the MIT License. See [LICENSE](LICENSE) for more details.
