# git-ai [![Rust](https://github.com/oleander/git-ai/actions/workflows/ci.yml/badge.svg)](https://github.com/oleander/git-ai/actions/workflows/ci.yml) [![Crates.io](https://img.shields.io/crates/v/git-ai.svg)](https://crates.io/crates/git-ai) [![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Git AI integrates ChatGPT with git hooks to automatically generate commit messages from your staged files. Leave your commit message blank, and Git AI will take it from there.

- Utilizes the OpenAI Assistant API, fine-tuned to convert git diffs into comprehensive commit messages.
- Allocates a distinct thread for each project, enabling the assistant to remember context and enhance performance with each commit.
- Establishes an isolated assistant instance on your computer, facilitating learning from all your projects and improving the quality of commit messages across your local development environment.

## TL;DR

```bash
# Install the binary
cargo install git-ai

# Configure the OpenAI API key
git-ai config set openai-api-key <your key>

# While in a git repository, install the hook
git-ai hook install

# Stage your changes and commit & that's it!
git commit --all --no-edit
```

## Installation

### From Crates.io

1. Ensure Rust and Cargo are installed on your system.
2. `cargo install git-ai`
3. `git-ai config set openapi-api-key <api-key>`
4. `git-ai hook install`

### From Source

1. Ensure Rust and Cargo are installed on your system.
2. Clone the Git AI repository: `git clone https://github.com/oleander/git-ai`
3. Change to the project directory: `cd git-ai`.
4. Build & install the binary: `cargo install --path .`

## Usage

- Install the binary as per the instructions above.
- Use `git ai hook install` to set up the Git hook.
- Set the OpenAI API key with `git-ai config set openapi-api-key <api-key>`.
- Use `git ai examples` to see some examples based on your previous commits. This does not change anything in your repository.

## CLI Options

### Configuration

Use `git-ai config set` followed by:

- `api-key <api-key>`: Set the OpenAI API key.
- `max-tokens <max-tokens>`: Set the maximum characters for `git diff` passed to OpenAI (default is 3500).
- `timeout <timeout>`: Set the maximum time in seconds to wait for OpenAI's response (default is 30).
- `language <language>`: Choose the model language (default is `en`).

### Hooks

Use `git-ai hook` followed by:

- `install`: Install the Git hook.
- `uninstall`: Uninstall the Git hook.

## Testing

* Execute `cargo test` to run the test suite.
* Use [act](https://github.com/nektos/act)

## License

This project is under the MIT License. For more details, see the [LICENSE](LICENSE) file.
