<p align="center">
  <img src="https://raw.githubusercontent.com/PKief/vscode-material-icon-theme/ec559a9f6bfd399b82bb44393651661b08aaf7ba/icons/folder-markdown-open.svg" width="100" alt="project-logo">
</p>
<p align="center">
    <h1 align="center">GIT-AI</h1>
</p>
<p align="center">
    <em>Commit with clarity, code with confidence.</em>
</p>
<p align="center">
	<img src="https://img.shields.io/github/license/oleander/git-ai?style=default&logo=opensourceinitiative&logoColor=white&color=0080ff" alt="license">
	<img src="https://img.shields.io/github/last-commit/oleander/git-ai?style=default&logo=git&logoColor=white&color=0080ff" alt="last-commit">
	<img src="https://img.shields.io/github/languages/top/oleander/git-ai?style=default&color=0080ff" alt="repo-top-language">
	<img src="https://img.shields.io/github/languages/count/oleander/git-ai?style=default&color=0080ff" alt="repo-language-count">
<p>
<p align="center">
	<!-- default option, no dependency badges. -->
</p>

<br><!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary><br>

- [Overview](#overview)
- [Features](#features)
- [Repository Structure](#repository-structure)
- [Modules](#modules)
- [Getting Started](#getting-started)
  - [Installation](#installation)
  - [Usage](#usage)
  - [Tests](#tests)
- [Project Roadmap](#project-roadmap)
- [Contributing](#contributing)
- [License](#license)
- [Acknowledgments](#acknowledgments)
</details>
<hr>

##  Overview

The git-ai project leverages AI to automate commit message generation within Git repositories. With a focus on enhancing productivity, it streamlines interactions by offering functionalities such as hook implementation, commit message styling, and configuration management. By facilitating seamless integration with GitHub Actions for CI/CD pipelines, git-ai ensures efficient testing and deployment processes. Through its ability to fine-tune commit messages and generate examples based on diffs, the project brings value by optimizing software development workflows and promoting clean code practices.

---

##  Features

|     | Feature           | Description                                                                                                                                                                                                                                                                                                     |
| --- | ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ⚙️   | **Architecture**  | The project is built using Rust and Docker, utilizing GitHub Actions for CI/CD. It incorporates a modular approach with clear separation of concerns leveraging Rust's capabilities for efficient development. Rust toolchain nightly ensures compatibility with the latest language features and improvements. |
| 🔩   | **Code Quality**  | The codebase maintains high quality with clear formatting standards enforced by rustfmt. It follows best practices for Rust development, demonstrating clean code structure and readability. Automated workflows such as CI/CD pipelines ensure continuous code quality checks.                                 |
| 📄   | **Documentation** | Documentation is well-presented with details on project setup, usage, and configurations. It includes descriptive comments within the codebase aiding in understanding the project logic and functionalities. README.md provides a comprehensive guide for contributors and users.                              |
| 🔌   | **Integrations**  | Key integrations include GitHub Actions for CI/CD automation, Rust toolchain for nightly builds, and external dependencies like Clap, Tokio, and Reqwest for handling interactions. Docker is also utilized for portability and simplifying dependency management.                                              |
| 🧩   | **Modularity**    | The project exhibits high modularity with distinct modules for committing, configuring, hooking, and styling. This modular design allows for easy maintenance, scalability, and reusability of components within the codebase.                                                                                  |
| 🧪   | **Testing**       | Testing frameworks and tools are not explicitly mentioned in the provided details. However, the project likely employs Rust testing frameworks like `cargo test` for unit and integration testing ensuring code reliability.                                                                                    |
| ⚡️   | **Performance**   | The project focuses on efficiency and speed in generating AI-powered commit messages. Utilizing Rust's performance optimizations and asynchronous capabilities with Tokio, it aims to provide fast and resource-efficient operations.                                                                           |
| 🛡️   | **Security**      | Security measures include handling access controls in repository interactions. The codebase likely follows Rust best practices for secure coding, maintaining data integrity, and preventing vulnerabilities. Error handling mechanisms ensure robustness against potential security threats.                   |
| 📦   | **Dependencies**  | Key external libraries and dependencies include Clap for command-line parsing, Tokio for asynchronous operations, Reqwest for HTTP client, all enhancing the project's capabilities. Rust toolchain nightly and GitHub Actions integration are crucial for development and automation.                          |

---

##  Repository Structure

```sh
└── git-ai/
    ├── .github
    │   └── workflows
    ├── Cargo.lock
    ├── Cargo.toml
    ├── Dockerfile
    ├── JUST-README.md
    ├── Justfile
    ├── LICENSE
    ├── README.md
    ├── resources
    │   ├── demo.cast
    │   └── demo.gif
    ├── rust-toolchain.toml
    ├── rustfmt.toml
    ├── scripts
    │   └── release
    ├── src
    │   ├── bin
    │   ├── commit.rs
    │   ├── config.rs
    │   ├── examples.rs
    │   ├── hook.rs
    │   ├── install.rs
    │   ├── lib.rs
    │   ├── main.rs
    │   ├── style.rs
    │   └── uninstall.rs
    ├── tests
    │   ├── common.rs
    │   └── patch_test.rs
    └── tools
        ├── demo.sh
        └── test.sh
```

---

##  Modules

<details closed><summary>.</summary>

| File                                                                                      | Summary                                                                                                                                                                                                                                                          |
| ----------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Dockerfile](https://github.com/oleander/git-ai/blob/master/Dockerfile)                   | Builds** Rust project, utilizes Docker for Rust binary creation, copies files, installs Git, sets up non-root user. Enhances project portability and simplifies dependencies management.                                                                         |
| [rust-toolchain.toml](https://github.com/oleander/git-ai/blob/master/rust-toolchain.toml) | Sets Rust toolchain to nightly within the repository. Maintains compatibility with Rusts latest features and improvements. Crucial for ensuring optimal development environment for the project.                                                                 |
| [Justfile](https://github.com/oleander/git-ai/blob/master/Justfile)                       | Enables running release and testing scripts using Docker images. Provides local installation, GitHub Actions setup, and Docker build/run commands for the `git-ai` repository. Facilitates seamless development and deployment workflows.                        |
| [Cargo.toml](https://github.com/oleander/git-ai/blob/master/Cargo.toml)                   | Automates Git commit messages using AI powered by ChatGPT. Generates messages based on staged files. Dependencies include Clap, Tokio, Reqwest, and more for handling interactions. File structure includes binaries for main functionality and related modules. |
| [rustfmt.toml](https://github.com/oleander/git-ai/blob/master/rustfmt.toml)               | Optimize Rust code formatting for enhanced readability and maintainability with specific layout and style configurations ensuring consistency across the project files.                                                                                          |

</details>

<details closed><summary>.github.workflows</summary>

| File                                                                              | Summary                                                                                                                                                                                                                                               |
| --------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [cd.yml](https://github.com/oleander/git-ai/blob/master/.github/workflows/cd.yml) | Automates CI/CD pipeline with GitHub Actions, enabling seamless testing & deployment. Triggers builds on each push to the main branch, ensuring reliable software delivery. Integrates with the repositorys Rust codebase for continuous integration. |
| [ci.yml](https://github.com/oleander/git-ai/blob/master/.github/workflows/ci.yml) | Automates CI process, integrating tests and checks into the Git-AI repo. Runs on push/pull requests, ensuring code quality via Rust. Orchestrates workflow tasks via predefined steps for efficiency and quality assurance.                           |

</details>

<details closed><summary>resources</summary>

| File                                                                            | Summary                                                                                                                                                                                                                                                                                   |
| ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [demo.cast](https://github.com/oleander/git-ai/blob/master/resources/demo.cast) | This code file in the git-ai repository focuses on managing workflows using GitHub Actions. It automates various processes related to testing and deployment through CI/CD pipelines. It plays a crucial role in ensuring smooth integration and delivery of code changes in the project. |

</details>

<details closed><summary>src</summary>

| File                                                                            | Summary                                                                                                                                                                                                                                                                |
| ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [main.rs](https://github.com/oleander/git-ai/blob/master/src/main.rs)           | Defines CLI commands for Git-AI hook, configuration, and examples. Handles subcommands for installation, uninstallation, and setting configuration values like model, language, and OpenAI API key. Enables running examples of generated commit messages.             |
| [hook.rs](https://github.com/oleander/git-ai/blob/master/src/hook.rs)           | Implements file and path handling, extracts diffs and patches from Git repositories. Parses command-line arguments for commit messages. Handles errors related to repository access and commit message writing.                                                        |
| [style.rs](https://github.com/oleander/git-ai/blob/master/src/style.rs)         | Implements a trait to derive relative path from current directory for styling purposes in the parent repositorys architecture.                                                                                                                                         |
| [install.rs](https://github.com/oleander/git-ai/blob/master/src/install.rs)     | Implements a Git hook in the repository to prepare a commit message. Symlinks a binary to the hook file location, enabling successful hook creation. Handles error cases like missing binaries and existing Git hooks.                                                 |
| [commit.rs](https://github.com/oleander/git-ai/blob/master/src/commit.rs)       | Generates Git commit messages based on diffs, creating connections, runs, and responses through OpenAIs API. It manages sessions, errors, and repository interactions seamlessly, enhancing commit message quality and efficiency within the Git-AI project structure. |
| [config.rs](https://github.com/oleander/git-ai/blob/master/src/config.rs)       | Initializes and updates app configurations using environment variables and INI files, requiring specific subcommands. Creates, saves, and loads settings from a designated path with error handling.                                                                   |
| [uninstall.rs](https://github.com/oleander/git-ai/blob/master/src/uninstall.rs) | Uninstall script removes Git hook file if it exists, enhancing repo maintenance by promoting clean code practices. It leverages error handling and Git2 for efficient execution within the parent repositorys architecture.                                            |
| [examples.rs](https://github.com/oleander/git-ai/blob/master/src/examples.rs)   | Generates AI-powered commit message examples by analyzing the last commits in a Git repository. Configurable to limit tokens for diffs. Utilizes progress bars for visualization during processing.                                                                    |
| [lib.rs](https://github.com/oleander/git-ai/blob/master/src/lib.rs)             | Defines modules for committing, configuring, hooking, and styling within the git-ai repository, contributing to project organization and encapsulation.                                                                                                                |

</details>

<details closed><summary>src.bin</summary>

| File                                                                                          | Summary                                                                                                                                                                                                                                                                    |
| --------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [fine-tune-md.rs](https://github.com/oleander/git-ai/blob/master/src/bin/fine-tune-md.rs)     | Generates commit examples for a project by analyzing message content and diffs for best practices. Excludes non-essential paths for training data. Outputs a curated file with commit details for learning purposes.                                                       |
| [clear.rs](https://github.com/oleander/git-ai/blob/master/src/bin/clear.rs)                   | Repository` to interact with the repository. Entry point for initiating config changes.                                                                                                                                                                                    |
| [fine-tune.rs](https://github.com/oleander/git-ai/blob/master/src/bin/fine-tune.rs)           | Extracts concise commit messages from Git diffs based on specified criteria.-Utilizes commit data to generate structured commit messages.-Segregates commits into training and validation sets for further processing.-Implements exclusion logic for specific file paths. |
| [hook.rs](https://github.com/oleander/git-ai/blob/master/src/bin/hook.rs)                     | Generates commit messages based on diffs in the repository, utilizing a progress bar to indicate processing status. Handles various scenarios for creating and amending commits while ensuring session data is saved to the repository.                                    |
| [fine-tune-json.rs](https://github.com/oleander/git-ai/blob/master/src/bin/fine-tune-json.rs) | Generates fine-tune data from git history by creating examples for AI training. Excludes specific file paths and criteria for more accurate data sets. Helps in optimizing AI training data quality.                                                                       |

</details>

<details closed><summary>scripts</summary>

| File                                                                      | Summary                                                                                                                                                                                                        |
| ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [release](https://github.com/oleander/git-ai/blob/master/scripts/release) | Automates version release process by updating dependencies, committing changes, tagging version, and pushing changes to the main branch. Integrated with CI/CD for streamlined open-source project management. |

</details>

---

##  Getting Started

**System Requirements:**

* **Rust**: `version x.y.z`

###  Installation

<h4>From <code>source</code></h4>

> 1. Clone the git-ai repository:
>
> ```console
> $ git clone https://github.com/oleander/git-ai
> ```
>
> 2. Change to the project directory:
> ```console
> $ cd git-ai
> ```
>
> 3. Install the dependencies:
> ```console
> $ cargo build
> ```

###  Usage

<h4>From <code>source</code></h4>

> Run git-ai using the command below:
> ```console
> $ cargo run
> ```

###  Tests

> Run the test suite using the command below:
> ```console
> $ cargo test
> ```

---

##  Project Roadmap

- [X] `► INSERT-TASK-1`
- [ ] `► INSERT-TASK-2`
- [ ] `► ...`

---

##  Contributing

Contributions are welcome! Here are several ways you can contribute:

- **[Report Issues](https://github.com/oleander/git-ai/issues)**: Submit bugs found or log feature requests for the `git-ai` project.
- **[Submit Pull Requests](https://github.com/oleander/git-ai/blob/main/CONTRIBUTING.md)**: Review open PRs, and submit your own PRs.
- **[Join the Discussions](https://github.com/oleander/git-ai/discussions)**: Share your insights, provide feedback, or ask questions.

<details closed>
<summary>Contributing Guidelines</summary>

1. **Fork the Repository**: Start by forking the project repository to your github account.
2. **Clone Locally**: Clone the forked repository to your local machine using a git client.
   ```sh
   git clone https://github.com/oleander/git-ai
   ```
3. **Create a New Branch**: Always work on a new branch, giving it a descriptive name.
   ```sh
   git checkout -b new-feature-x
   ```
4. **Make Your Changes**: Develop and test your changes locally.
5. **Commit Your Changes**: Commit with a clear message describing your updates.
   ```sh
   git commit -m 'Implemented new feature x.'
   ```
6. **Push to github**: Push the changes to your forked repository.
   ```sh
   git push origin new-feature-x
   ```
7. **Submit a Pull Request**: Create a PR against the original project repository. Clearly describe the changes and their motivations.
8. **Review**: Once your PR is reviewed and approved, it will be merged into the main branch. Congratulations on your contribution!
</details>

<details closed>
<summary>Contributor Graph</summary>
<br>
<p align="center">
   <a href="https://github.com{/oleander/git-ai/}graphs/contributors">
      <img src="https://contrib.rocks/image?repo=oleander/git-ai">
   </a>
</p>
</details>

---

##  License

This project is protected under the [SELECT-A-LICENSE](https://choosealicense.com/licenses) License. For more details, refer to the [LICENSE](https://choosealicense.com/licenses/) file.

---

##  Acknowledgments

- List any resources, contributors, inspiration, etc. here.

[**Return**](#-overview)

---
