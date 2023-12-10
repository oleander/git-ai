# Git AI Library [![Rust](https://github.com/oleander/git-ai/actions/workflows/rust.yml/badge.svg)](https://github.com/oleander/git-ai/actions/workflows/rust.yml)

## Overview

Git AI is a Rust library that integrates with Git, leveraging OpenAI's GPT-4 model to automatically generate commit messages from code changes. 

This crate installs a `prepare-commit-msg` Git hook in your repository. When you commit without a message, Git AI uses ChatGPT to craft a commit message based on the staged files.


## Installation

### Pre-Built Binaries

1. `cargo binstall cargo-binstall`
2. `cargo binstall git-ai`
 
### From Source

1. Ensure Rust and Cargo are installed on your system.
2. Clone the Git AI repository: `git clone https://github.com/oleander/git-ai`
3. Change to the project directory: `cd git-ai`.
4. Build the hook: `cargo build --release --bin git-ai-hook`
5. Build & install the binary: `cargo install --path . --bin git-ai`

## Usage

- Install the binary as per the instructions above.
- Use `git ai hook install` to set up the Git hook.
- Set the OpenAI API key with `git-ai config set api-key <api-key>`.

## CLI Options

### Configuration

Use `git-ai config set` followed by:

- `api-key <api-key>`: Set the OpenAI API key.
- `max-tokens <max-tokens>`: Set the maximum characters for `git diff` passed to OpenAI (default is 5000).
- `timeout <timeout>`: Set the maximum time in seconds to wait for OpenAI's response (default is 30).
- `language <language>`: Choose the model language (default is `en`).

### Hooks

Use `git-ai hook` followed by:

- `install`: Install the Git hook.
- `uninstall`: Uninstall the Git hook.
  
## Testing

Execute `cargo test` to run the test suite.

## License

This project is under the MIT License. For more details, see the [LICENSE](LICENSE) file.

## Pre-Publish Checklist

- [x] Decide on an appropriate name for the binary.
- [x] Update the README with installation and testing instructions.
- [x] Ensure continuous integration (CI) passes.
  - [x] Look into ways to simplify the CI process.
- [x] Define and document configuration options.
- [x] Implement a feature where CTRL-C resets the terminal.
- [ ] Change the command-line interface (CLI) to use subcommands:
  - [ ] `git ai hook install`
  - [ ] `git ai hook uninstall`
- [ ] Publish the crate to crates.io
- [ ] Get rid of the main.rs as binary