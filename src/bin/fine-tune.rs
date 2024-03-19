use std::io::Write;
use std::fs::File;

use git2::{Commit, DiffFormat, DiffOptions, Repository};
use anyhow::{Context, Result};
use serde_json::json;

static PROMPT: &str = "Your role is to create concise git commit messages based on user-provided git diffs. When crafting these messages: - Focus on detailing the changes and reasons behind them, ensuring clarity and relevance. - Avoid including irrelevant or unnecessary details, such as translations, to maintain focus on the core changes. Your responses should be direct and immediately usable in a git commit, crafted in present tense to fit git conventions. You work primarily with git diffs, interpreting them to generate meaningful commit messages that succinctly summarize the changes.";

fn main() -> Result<()> {
  env_logger::init();

  let max_tokens = 16385;
  let validate_file_name = "validate.jsonl";
  let train_file_name = "train.jsonl";
  let max_commits = 10;

  log::info!("Creating fine-tune file with {} commits and {} tokens", max_commits, max_tokens);

  let repo = Repository::open(".").context("Failed to open git repository")?;
  let config = repo.config().context("Couldn't access repository config")?;
  let user_email = config.get_string("user.email").context("Couldn't get user email")?;
  let mut revwalk = repo.revwalk().context("Failed to create Revwalk")?;
  let mut validate_file = File::create(validate_file_name).context("Failed to create file")?;
  let mut train_file = File::create(train_file_name).context("Failed to create file")?;

  validate_file.write_all(b"").context("Failed to write to file")?;
  train_file.write_all(b"").context("Failed to write to file")?;

  revwalk.push_head().expect("Failed to push head");

  let mut curr_size = 0;
  let mut commit_count = 0;
  let mut result = vec![];

  for oid in revwalk {
    let oid = oid.context("Failed to get oid")?;
    let commit = repo.find_commit(oid).context("Couldn't find commit")?;

    if commit.parent_count() > 1 {
      continue;
    }

    let weight = if commit.author().email() == Some(&user_email) {
      1
    } else if commit.committer().email() == Some(&user_email) {
      1
    } else {
      0
    };

    let Ok(Some(content)) = generate_commit_diff(&repo, &commit) else {
      continue;
    };

    let Some(commit) = commit.message() else {
      continue;
    };

    if commit.starts_with("Merge") {
      continue;
    }

    if commit.len() > 80 {
      continue;
    }

    // Check if it contains a new line
    if commit.trim().contains("\n") {
      continue;
    }

    if commit.contains("[") && commit.contains("]") {
      continue;
    }

    let message = json!({
      "messages": [
        { "role": "assistant", "content": commit.trim() },
        { "role": "user", "content": content.trim() },
        { "role": "system", "content": PROMPT }
      ]
    });

    let content = serde_json::to_string(&message)?;
    curr_size += content.split_whitespace().count();

    if curr_size > max_tokens {
      log::warn!("Max tokens reached: {}", max_tokens);
      break;
    }

    commit_count += 1;
    result.push(message);
  }

  let train_result = result[..(result.len() / 2)].to_vec();
  for (i, message) in train_result.iter().enumerate() {
    let content = serde_json::to_string(&message)?;
    if i > 0 {
      train_file.write_all(b"\n").context("Failed to write to file")?;
    }
    train_file.write_all(content.as_bytes()).context("Failed to write to file")?;
  }

  let validate_result = result[(result.len() / 2)..].to_vec();
  for (i, message) in validate_result.iter().enumerate() {
    let content = serde_json::to_string(&message)?;
    if i > 0 {
      validate_file.write_all(b"\n").context("Failed to write to file")?;
    }
    validate_file.write_all(content.as_bytes()).context("Failed to write to file")?;
  }

  log::info!("Wrote {} commits to train file and {} commits to validate file", commit_count / 2, commit_count / 2);

  Ok(())
}

fn should_exclude_path(file_path: &str) -> bool {
  let exclude_patterns = vec![
    "/docs/", "/documentation/", "/guides/", // Documentation
    "/assets/", "/images/", "/graphics/", "/designs/", // Assets and design-related files
    "Gemfile", "Gemfile.lock", // Dependency files
    "/config/", "/settings/", "/initializers/", // Configuration files
    "/vendor/", "/third-party/", "/external/",   // Third-party and vendor code
    "/submodules/", // Git submodules
    "/.github/", "/.gitignore", "/.gitmodules", "/.gitattributes", // Git and GitHub specific files
    "/.gitlab-ci.yml", "/.travis.yml", "/appveyor.yml", // CI/CD configuration files
    "/Dockerfile", "/docker-compose.yml", "/.dockerignore", // Docker files
    "/.editorconfig", "/.rubocop.yml", "/.eslintignore", "/.eslintrc", // Linter and editor configuration
    "/test/", "/spec/", "/tests/", "/specs/", // Test files and directories
    "/locales/", "/i18n/", // Localization files
    "/logs/", "/tmp/",    // Logs and temporary files
    "/public/", // Public assets
    "/node_modules/", "/package.json", "/yarn.lock", // Node.js specific files
    "/.env", "/.env.example", // Environment files
    "/db/schema.rb", "/db/migrate/", // Database schema and migrations
    "/scripts/", "/tools/", // Utility scripts and tools
    "/CHANGELOG", "/LICENSE", "/README.md", // Project meta-files
  ];

  exclude_patterns.iter().any(|pattern| file_path.contains(pattern))
}

fn generate_commit_diff(repo: &Repository, commit: &Commit) -> Result<Option<String>> {
  let parent = commit.parents().next().unwrap_or_else(|| commit.clone());
  let tree = commit.tree().expect("Couldn't get commit tree");
  let parent_tree = parent.tree().expect("Couldn't get parent tree");
  let mut opts = DiffOptions::new();
  opts
    .ignore_whitespace_change(true)
    .recurse_untracked_dirs(false)
    .recurse_ignored_dirs(false)
    .ignore_whitespace_eol(true)
    .ignore_blank_lines(true)
    .include_untracked(false)
    .ignore_whitespace(true)
    .indent_heuristic(false)
    .ignore_submodules(true)
    .include_ignored(false)
    .interhunk_lines(0)
    .context_lines(0)
    .patience(true)
    .minimal(true);

  let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut opts)).context("Failed to get diff")?;

  let mut patch: Vec<u8> = Vec::new();

  #[rustfmt::skip]
  diff.print(DiffFormat::Patch, |delta, _, line| {
    // Ignore if line is a binary file
    if line.origin() == 'B' {
      return false;
    }

    let file_path = delta.new_file().path().unwrap_or_else(|| delta.old_file().path().unwrap());

    if should_exclude_path(file_path.to_str().unwrap()) {
      return false;
    }

    let content = line.content();
    patch.extend_from_slice(content);

    true
  }).context("Failed to print diff")?;

  let content = String::from_utf8(patch).context("Failed to convert patch to string")?;
  if content.split_whitespace().count() > 600 { Ok(None) } else { Ok(Some(content)) }
}
