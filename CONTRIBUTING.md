# Contributing to Git AI

We're thrilled you're interested in contributing to Git AI! This document provides guidelines for making contributions to this project. By participating in this project, you agree to abide by its terms and contribute to the improvement of automated commit messages using AI.

## Getting Started

### 1. Fork the Repository

Start by forking the project repository to your GitHub account. This creates a personal copy where you can work on changes without affecting the original project.

### 2. Clone Your Fork

Clone your fork to your local machine to start working on the changes.

```bash
git clone https://github.com/oleander/git-ai.git
cd git-ai
```

### 3. Create a New Branch

For each new feature or bug fix, create a new branch based on the `main` branch. This keeps your changes organized and simplifies the process of integrating your contributions.

```bash
git checkout -b feature/my-new-feature
```
or
```bash
git checkout -b fix/my-bug-fix
```

## Making Changes

### Write Clean Code

- Follow the coding standards already in place within the project.
- Write meaningful commit messages that clearly describe your changes.
- Include comments in your code where necessary to explain complex logic.

### Test Your Changes

Before submitting a pull request, make sure your changes do not break the existing functionality. Run the project's test suite and, if possible, add new tests to cover your contributions.

```bash
cargo test
```

### Update Documentation

If you're adding a new feature or changing existing functionality, update the README.md and any relevant documentation. Clear, concise documentation ensures that everyone can benefit from your contributions.

## Submitting a Pull Request

1. Push your changes to your fork:

    ```bash
    git push origin feature/my-new-feature
    ```

    or

    ```bash
    git push origin fix/my-bug-fix
    ```

2. Go to your fork on GitHub and click the "Pull Request" button to submit your changes.

3. Provide a concise and clear description of your changes, explaining the purpose of your contributions and any relevant details.

4. Submit your pull request to the main project for review.

## Code Review Process

The project maintainers will review your pull request. This process helps ensure the quality and consistency of the project. You may receive feedback or requests for changes to your contributions. This is a normal part of the contribution process, and we encourage you to see it as an opportunity to learn and improve.

## Community Guidelines

We aim to maintain a respectful and collaborative environment. We expect all contributors to:

- Be respectful of different viewpoints and experiences.
- Gracefully accept constructive criticism.
- Focus on what is best for the community and the project.

Harassment, derogatory comments, and personal attacks are not tolerated in this project.

## Questions?

If you have any questions or need further clarification about contributing, please open an issue with your question.

---

Thank you for contributing to Git AI! Your efforts help make this project better for everyone.
