// Hook: prepare-commit-msg

use std::path::Path;

use git2::{DiffOptions, Repository, RepositoryOpenFlags};
use ai::commit::generate_commit;
use anyhow::{Context, Result};

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

// TOOD: Copy of src/bin/hook.rs
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

pub async fn run(args: &clap::ArgMatches) -> Result<()> {
  let current_dir = std::env::current_dir().context("Failed to get current directory")?;
  let repo = Repository::open_ext(&current_dir, RepositoryOpenFlags::empty(), Vec::<&Path>::new())?;
  let max_tokens: usize = *args.get_one("max-tokens").context("Failed to get max tokens")?;
  let max_commits: usize = *args.get_one("max-commits").context("Failed to get max commits")?;
  let commits = repo.get_last_n_commits(max_commits).context("Failed to get last commit")?;

  println!("Examples of generated commit messages from the last {} commits:", commits.len());
  for (index, commit) in commits.iter().enumerate() {
    let commit_message = generate_commit(commit.show(&repo, max_tokens)?).await?;
    println!("Commit #{}:", index + 1);
    println!("\tGenerated commit message: {}", commit_message);
    println!("\tOriginal commit message: {}", commit.message().unwrap_or(""));
  }

  Ok(())
}
