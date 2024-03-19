use std::io::Write;
use std::fs::File;

use git2::{Commit, DiffOptions, Repository};
use anyhow::{Context, Result};
use indicatif::ProgressBar;
use ai::hook::PatchDiff;
use serde_json::json;

static PROMPT: &str = "Your role is to create concise git commit messages based on user-provided git diffs. When crafting these messages: - Focus on detailing the changes and reasons behind them, ensuring clarity and relevance. - Avoid including irrelevant or unnecessary details, such as translations, to maintain focus on the core changes. Your responses should be direct and immediately usable in a git commit, crafted in present tense to fit git conventions. You work primarily with git diffs, interpreting them to generate meaningful commit messages that succinctly summarize the changes.";

fn main() -> Result<()> {
  let pb = ProgressBar::new_spinner();
  let file_name = "file-tune.json";
  let limit = 1000;

  // set number of items in pb
  pb.set_length(limit as u64);
  log::info!("Creating fine-tune file with {} commits", limit);

  let repo = Repository::open(".").context("Failed to open git repository")?;
  let config = repo.config().context("Couldn't access repository config")?;
  let user_email = config.get_string("user.email").context("Couldn't get user email")?;
  let mut revwalk = repo.revwalk().context("Failed to create Revwalk")?;
  let mut file = File::create(file_name).context("Failed to create file")?;

  file.write_all(b"").context("Failed to write to file")?;

  revwalk.push_head().expect("Failed to push head");

  for oid in revwalk.take(limit) {
    let oid = oid.context("Failed to get oid")?;
    let commit = repo.find_commit(oid).context("Couldn't find commit")?;
    let commit = if commit.author().email() == Some(&user_email) {
      commit
    } else if commit.committer().email() == Some(&user_email) {
      commit
    } else {
      continue;
    };

    let message = json!({
      "messages": [
        { "role": "system", "content": PROMPT },
        { "role": "user", "content": generate_commit_diff(&repo, &commit) },
        { "role": "assistant", "content": commit.message().unwrap_or_default() }
      ]
    });

    file.write_all(serde_json::to_string_pretty(&message)?.as_bytes()).context("Failed to write to file")?;
    pb.inc(1);
  }

  pb.finish_with_message("Done");

  log::info!("File {} created with {} commits", file_name, limit);

  Ok(())
}

fn generate_commit_diff(repo: &Repository, commit: &Commit) -> String {
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
  let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut opts)).expect("Couldn't generate diff");
  let patches = diff.to_patch(5000).expect("Couldn't generate patch");
  String::from_utf8(patches.as_bytes().to_vec()).unwrap()
}
