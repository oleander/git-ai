# git-ai [![Rust](https://github.com/oleander/git-ai/actions/workflows/ci.yml/badge.svg)](https://github.com/oleander/git-ai/actions/workflows/ci.yml)

Git AI uses ChatGPT and git hook to generate commit messages based on the staged files. Leave the commit message empty and let Git AI do the work for you!

## TL;DR

```bash
cargo binstall cargo-binstall
cargo binstall git-ai
git ai config set openapi-api-key <api-key>
cd <your-git-repo>
git ai hook install
# make a change
git add .
git commit --no-edit
```

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
- Set the OpenAI API key with `git-ai config set openapi-api-key <api-key>`.

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
