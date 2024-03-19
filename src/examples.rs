// Hook: prepare-commit-msg

use std::path::Path;
use std::time::Duration;

use colored::Colorize;
use git2::{DiffOptions, Repository, RepositoryOpenFlags};
use anyhow::{Context, Result};
use ai::config::APP;
use ai::commit;

const MAX_NUMBER_OF_COMMITS: usize = 5;

trait RepositoryExt {
  fn get_last_n_commits(&self, max_commits: usize) -> Result<Vec<git2::Commit>, git2::Error>;
}

impl RepositoryExt for Repository {
  fn get_last_n_commits(&self, max_commits: usize) -> Result<Vec<git2::Commit>, git2::Error> {
    let mut revwalk = self.revwalk()?;
    revwalk.push_head()?;
    Ok(
      revwalk
        .take(max_commits)
        .map(move |id| self.find_commit(id.unwrap()).expect("Failed to find commit"))
        .collect()
    )
  }
}

// TODO: Duplicate code from src/commit.rs
trait CommitExt {
  fn show(&self, repo: &Repository, max_tokens: usize) -> Result<String, git2::Error>;
}

impl CommitExt for git2::Commit<'_> {
  fn show(&self, repo: &Repository, max_tokens: usize) -> Result<String, git2::Error> {
    let mut commit_info = "".to_string();
    let mut opts = DiffOptions::new();
    let tree = self.tree()?;
    let parent_tree = self.parent(0).ok().as_ref().map(|c| c.tree().ok()).flatten();
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))?;

    _ = diff
      .print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        commit_info.push_str(std::str::from_utf8(line.content()).unwrap());
        commit_info.len() < max_tokens
      })
      .ok();

    Ok(commit_info)
  }
}

use console::Style;
use indicatif::{ProgressBar, ProgressStyle};

pub async fn run(_args: &clap::ArgMatches) -> Result<()> {
  let max_tokens = APP.max_diff_tokens;

  let current_dir = std::env::current_dir().context("Failed to get current directory")?;
  let repo = Repository::open_ext(&current_dir, RepositoryOpenFlags::empty(), Vec::<&Path>::new())?;
  let commits = repo.get_last_n_commits(MAX_NUMBER_OF_COMMITS).context("Failed to get last commits")?;

  // Create and configure the progress bar
  let spinner_style = ProgressStyle::default_spinner()
    .tick_strings(&["-", "\\", "|", "/"])
    .template("{spinner:.blue} {msg}")
    .context("Failed to create progress bar style")?;

  let pb = ProgressBar::new_spinner();
  pb.set_style(spinner_style);
  pb.enable_steady_tick(Duration::from_millis(100));

  let header_style = Style::new().bold();
  println!("{}", header_style.apply_to("🛠️  AI-Generated Commit Message Examples"));

  for (index, commit) in commits.iter().enumerate() {
    pb.set_message(format!("Loading commit #{} ...\n", index + 1));
    let commit_message = commit::generate(commit.show(&repo, max_tokens)?, None).await?.response;
    pb.println(format!("Commit #{}:", index + 1));
    pb.println(format!("\tOriginal: {}", commit.message().unwrap_or_default().trim().italic()));
    pb.println(format!("\tGenerated: {}", commit_message.italic()));
  }

  Ok(())
}
