use std::io::Write;
use std::fs::File;

use git2::{Commit, DiffFormat, DiffOptions, Repository};
use anyhow::{Context, Result};
use indicatif::ProgressBar;
use ai::hook::PatchDiff;
use serde_json::json;

static PROMPT: &str = "Your role is to create concise git commit messages based on user-provided git diffs. When crafting these messages: - Focus on detailing the changes and reasons behind them, ensuring clarity and relevance. - Avoid including irrelevant or unnecessary details, such as translations, to maintain focus on the core changes. Your responses should be direct and immediately usable in a git commit, crafted in present tense to fit git conventions. You work primarily with git diffs, interpreting them to generate meaningful commit messages that succinctly summarize the changes.";

fn main() -> Result<()> {
  env_logger::init();

  let max_tokens = 16385;
  let file_name = "file-tune.json";
  let max_commits = 100;

  log::info!("Creating fine-tune file with {} commits and {} tokens", max_commits, max_tokens);

  let repo = Repository::open(".").context("Failed to open git repository")?;
  let config = repo.config().context("Couldn't access repository config")?;
  let user_email = config.get_string("user.email").context("Couldn't get user email")?;
  let mut revwalk = repo.revwalk().context("Failed to create Revwalk")?;
  let mut file = File::create(file_name).context("Failed to create file")?;

  file.write_all(b"").context("Failed to write to file")?;

  revwalk.push_head().expect("Failed to push head");

  let mut curr_size = 0;
  let mut commit_count = 0;

  for oid in revwalk.take(max_commits) {
    let oid = oid.context("Failed to get oid")?;
    let commit = repo.find_commit(oid).context("Couldn't find commit")?;
    let commit = if commit.author().email() == Some(&user_email) {
      commit
    } else if commit.committer().email() == Some(&user_email) {
      commit
    } else {
      continue;
    };

    let Some(content) = generate_commit_diff(&repo, &commit)? else {
      continue;
    };

    let Some(commit) = commit.message() else {
      continue;
    };

    let message = json!({
      "messages": [
        { "role": "assistant", "content": commit },
        { "role": "user", "content": content },
        { "role": "system", "content": PROMPT }
      ]
    });

    let content = serde_json::to_string_pretty(&message)?;
    curr_size += content.split_whitespace().count();

    if curr_size > max_tokens {
      log::warn!("Max tokens reached: {}", max_tokens);
      break;
    }

    commit_count += 1;
    file.write_all(content.as_bytes()).context("Failed to write to file")?;
  }

  log::info!("File {} created with {} commits", file_name, commit_count);

  Ok(())
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
  diff.print(DiffFormat::Patch, |_, _, line| {
    let content = line.content();
    patch.extend_from_slice(content);
    true
  }).context("Failed to print diff")?;

  let content = String::from_utf8(patch).context("Failed to convert patch to string")?;

  if content.split_whitespace().count() > 500 {
    Ok(None)
  } else {
    Ok(Some(content))
  }
}
