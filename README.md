# Git AI Library

## Overview

Git AI is a Rust library that integrates with Git, leveraging OpenAI's GPT-4 model to automatically generate commit messages from code changes. 

This crate installs a `prepare-commit-msg` Git hook in your repository. When you commit without a message, Git AI uses ChatGPT to craft a commit message based on the staged files.

## Installation

1. Ensure Rust and Cargo are installed on your system.
2. Clone the Git AI repository: `git clone [repository-url]`.
3. Change to the project directory: `cd git-ai`.
4. Build the project with: `cargo build --release`.
5. Install the Git AI binary: `cargo install --path .`.

## Usage

- Move to the Git repository where you want the hook installed.
- Use `git-ai hook install` to set up the Git hook.
- Set the OpenAI API key with `git-ai config set api-key <api-key>`.

## Configuration

Use `git-ai config set` followed by:

- `api-key <api-key>`: Set the OpenAI API key.
- `max-tokens <max-tokens>`: Set the maximum characters for `git diff` passed to OpenAI (default is 5000).
- `timeout <timeout>`: Set the maximum time in seconds to wait for OpenAI's response (default is 30).
- `language <language>`: Choose the model language (default is `en`).

## Testing

Execute `cargo test` to run the test suite.

## License

This project is under the MIT License. For more details, see the [LICENSE](LICENSE) file.

## Pre-Publish Checklist

- Decide on an appropriate name for the binary.
- Update the README with installation and testing instructions.
- Ensure continuous integration (CI) passes.
  - Look into ways to simplify the CI process.
- Define and document configuration options.
- Implement a feature where CTRL-C resets the terminal.