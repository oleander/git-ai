# git-ai [![Rust](https://github.com/oleander/git-ai/actions/workflows/ci.yml/badge.svg)](https://github.com/oleander/git-ai/actions/workflows/ci.yml) [![Crates.io](https://img.shields.io/crates/v/git-ai.svg)](https://crates.io/crates/git-ai) [![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Git AI leverages ChatGPT alongside git hooks to automate the generation of commit messages from staged files. Simply stage your files and leave the commit message blank — Git AI will handle the rest.

- Uses OpenAI's Assistant API which has been specifically fine-tuned to transform git diffs into descriptive commit messages.
- Maintains a unique thread for each project where git-ai is utilized, ensuring the assistant retains context and improves based on previous commits.
- Creates a single assistant instance for your computer, allowing for cross-project learning and more insightful commit messages from your local machine's activities.

<table>
  <tr>
    <!-- This cell contains the GIF -->
    <td style="width: 50%; text-align: center;">
      <img src="resources/demo.gif" alt="demo" style="max-width: 100%;"/>
    </td>
    <td style="width: 50%; vertical-align: top;">
      <pre><code>
cargo install git-ai
git-ai config set openapi-api-key &lt;key&gt;
cd &lt;your-git-repo&gt;
git-ai hook install
git add .
git commit --no-edit
      </code></pre>
    </td>
  </tr>
</table>


## Installation

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
