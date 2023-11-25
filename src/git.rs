#![allow(dead_code)]
#![allow(unused_imports)]

use git2::{
  Commit, Delta, Diff, DiffFormat, DiffOptions, Index, IndexAddOption, ObjectType, Oid, Repository, RepositoryInitOptions, RepositoryOpenFlags as Flag, StatusOptions, StatusShow
};
use anyhow::{anyhow, bail, Context, Result};
use log::{debug, error, info, trace, warn};
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use lazy_static::lazy_static;
use std::process::Command;
use std::path::Path;
use std::collections::HashSet;
use crate::chat;

pub struct Repo {
  repo: Arc<RwLock<Repository>>
}

impl Repo {
  pub fn new() -> Result<Self> {
    Self::new_with_path(".".to_string())
  }

  pub fn new_with_path(path: String) -> Result<Self> {
    let repo =
      Repository::open_ext(path, Flag::empty(), Vec::<&Path>::new())
        .with_context(|| format!("Failed to open the git repository at"))?;

    Ok(Repo {
      repo: Arc::new(RwLock::new(repo))
    })
  }

  pub fn diff_options() -> DiffOptions {
    let mut opts = DiffOptions::new();
    opts
      .enable_fast_untracked_dirs(true)
      .ignore_whitespace_change(true)
      .recurse_untracked_dirs(false)
      .recurse_ignored_dirs(false)
      .ignore_whitespace_eol(true)
      .ignore_blank_lines(true)
      .ignore_submodules(true)
      .include_untracked(false)
      .include_ignored(false)
      .interhunk_lines(0)
      .context_lines(0)
      .minimal(true)
      .patience(true)
      .indent_heuristic(false);
    opts
  }

  pub fn stats(&self) -> Result<git2::DiffStats> {
    let mut opts = Repo::diff_options();
    let repo = self.repo.read().expect("Failed to lock repo");
    let diff = repo.diff_tree_to_workdir_with_index(None, Some(&mut opts))?;
    diff.stats().context("Failed to get diff stats")
  }
  pub fn diff(&self, max_token_count: usize) -> Result<(String, Vec<String>)> {
    let repo = self.repo.read().expect("Failed to lock repo");
    let mut files = Vec::new();
    let mut diff_str = Vec::new();
    let mut opts = Repo::diff_options();

    let diff = match repo.head() {
      Ok(ref head) => {
        let tree = head
          .resolve()
          .context("Failed to resolve head")?
          .peel(ObjectType::Commit)
          .context("Failed to peel head")?
          .into_commit()
          .map_err(|_| anyhow!("Failed to resolve commit"))?
          .tree()
          .context("Failed to get tree")?;
        repo.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts))?
      },
      Err(_) => repo.diff_tree_to_workdir_with_index(None, Some(&mut opts))?
    };

    diff.foreach(
      &mut |delta, _| {
        if let Some(file) = delta.new_file().path() {
          let file_path = file.to_string_lossy().into_owned();
          files.push(file_path);
        }
        true
      },
      None,
      None,
      None
    )?;

    if files.is_empty() {
      bail!("No files to commit");
    }

    let mut length = 0;
    diff.print(git2::DiffFormat::Patch, |_, _, line| {
      let content = line.content();
      diff_str.extend_from_slice(content);
      let str = String::from_utf8(content.into()).unwrap_or_default();
      length += str.len();
      length < max_token_count
    }).ok();

    let mut diff_output =
      String::from_utf8(diff_str).expect("Diff output is not valid UTF-8");

    diff_output.truncate(max_token_count);

    debug!("Diff: {}", diff_output);

    Ok((diff_output, files))
  }

  pub async fn commit(&self, add_all: bool) -> Result<()> {
    debug!("[commit] Committing with message");

    let repo = self.repo.read().expect("Failed to lock repo");
    let mut index = repo.index().expect("Failed to get index");

    if add_all {
      debug!("Adding all files to index(--all)");

      index
        .add_all(["*"], IndexAddOption::DEFAULT, None)
        .context("Failed to add all files to index")?;
      index.write().context("Failed to write index")?;
    }

    let (diff, _) = self.diff(1000)?;
    let oid = index.write_tree().context("Failed to write tree")?;
    let tree = repo.find_tree(oid).context("Failed to find tree")?;
    let signature = repo.signature().context("Failed to get signature")?;
    let message = chat::suggested_commit_message(diff).await?;

    match repo.head() {
      Ok(ref head) => {
        let parent = head
          .resolve()
          .context("Failed to resolve head")?
          .peel(ObjectType::Commit)
          .context("Failed to peel head")?
          .into_commit()
          .map_err(|_| anyhow!("Failed to resolve parent commit"))?;

        repo
          .commit(
            Some("HEAD"),
            &signature,
            &signature,
            &message,
            &tree,
            &[&parent]
          )
          .context("Failed to commit (1)")?;
      },
      Err(_) => {
        repo
          .commit(Some("HEAD"), &signature, &signature, &message, &tree, &[])
          .context("Failed to commit (2)")?;
      }
    }

    Ok(())
  }
}

pub fn repo() -> Repo {
  Repo::new().expect("Failed to initialize git repository")
}

#[cfg(test)]
mod tests {
  use git2::{Commit, IndexAddOption, ObjectType, Repository};
  use anyhow::{anyhow, bail, Context, Result};
  use std::path::{Path, PathBuf};
  use std::process::Command;
  use tempfile::TempDir;
  use crate::git::Repo;
  use std::io::Write;
  use std::fs::File;
  use log::info;

  pub struct Git2Helpers {
    dir: TempDir
  }

  impl Git2Helpers {
    pub fn new() -> (Self, Repo) {
      let helper = Git2Helpers {
        dir: TempDir::new().expect("Could not create temp dir")
      };

      helper.git(&["init"]);

      let repo = Repo::new_with_path(helper.str_path().to_string())
        .expect("Could not open repo");

      (helper, repo)
    }

    pub fn path(&self) -> &Path {
      self.dir.path()
    }

    pub fn random_content() -> String {
      format!("Random content {}", rand::random::<u8>())
    }

    pub fn replace_file(&self, file_name: &str) {
      info!("FRom: {:?}", self.read_file(file_name));
      let random_content = Self::random_content();
      info!("Random content: {}", random_content);
      let file_path = self.path().join(file_name);
      std::fs::write(&file_path, random_content).unwrap();
      info!("To: {:?}", self.read_file(file_name));
    }

    pub fn read_file(&self, file_name: &str) -> String {
      let file_path = self.path().join(file_name);
      std::fs::read_to_string(&file_path).expect("Could not read file")
    }

    pub fn create_file(&self, file_name: &str) {
      let random_content = Self::random_content();
      let file_path = self.path().join(file_name);
      let mut file = File::create(&file_path).expect("Could not create file");
      file
        .write_all(random_content.as_bytes())
        .expect("Could not write to file");
    }

    pub fn delete_file(&self, file_name: &str) {
      let file_path = self.path().join(file_name);
      std::fs::remove_file(&file_path).expect("Could not delete file");
    }

    pub fn str_path(&self) -> &str {
      self.path().to_str().unwrap()
    }

    pub fn status(&self) -> Result<String> {
      self.git(&["status"])
    }

    pub fn stage_file(&self, file_name: &str) -> Result<String> {
      self.git(&["add", file_name])
    }

    pub fn debug(&self) -> Result<()> {
      let status = self.status()?;
      info!("Status: {}", status);
      let diff = self.git(&["diff", "--cached"])?;
      info!("Diff: {}", diff);
      Ok(())
    }

    fn git(&self, args: &[&str]) -> Result<String> {
      let output = Command::new("git")
        .args(args)
        .env("OVERCOMMIT_DISABLE", "1")
        .current_dir(self.path())
        .output()
        .context("Could not run git command")?;

      if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Git command failed: {}", stderr);
      }

      Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn stage_deleted_file(&self, file_name: &str) -> Result<String> {
      self.git(&["add", file_name])
    }

    pub fn commit(&self) -> Result<String> {
      let message = format!("Commit {}", rand::random::<u8>());
      self.git(&["commit", "-m", &message])
    }
  }

  fn setup() {
    _ = env_logger::builder().is_test(true).try_init();
  }

  #[test]
  fn file_replacement() {
    setup();

    let (helpers, repo) = Git2Helpers::new();

    /*  A file is created and committed */
    helpers.create_file("test.txt");
    helpers.stage_file("test.txt");
    helpers.commit();
    let res = repo.diff(usize::MAX);
    assert!(res.is_err());

    /* Reset */
    helpers.commit();

    /* A new file is created and committed */
    helpers.create_file("other.txt");
    helpers.stage_file("other.txt");
    let (_, files) = repo.diff(usize::MAX).expect("Could not generate diff");
    assert_eq!(files, vec!["other.txt"]);

    /* Reset */
    helpers.commit();

    /* The file is modified and committed */
    helpers.replace_file("test.txt");
    helpers.stage_file("test.txt");
    let (_, files) = repo.diff(usize::MAX).expect("Could not generate diff");
    assert_eq!(files, vec!["test.txt"]);

    // /* Reset */
    helpers.commit();

    // /* The file is modified again without staging */
    helpers.create_file("new.txt");
    let res = repo.diff(usize::MAX);
    assert!(res.is_err());
  }

  // Test case for adding a new file and committing it
  #[test]
  fn add_and_commit_new_file() {
    setup();
    let (helpers, repo) = Git2Helpers::new();
    helpers.create_file("new_file.txt");
    helpers.stage_file("new_file.txt");
    helpers.commit();
    let res = repo.diff(usize::MAX);
    assert!(res.is_err());
  }

  // Test case for deleting a file and committing the deletion
  #[test]
  fn delete_and_commit_file() {
    setup();
    let (helpers, repo) = Git2Helpers::new();
    helpers.create_file("deletable_file.txt");
    helpers.stage_file("deletable_file.txt");
    helpers.commit();
    helpers.delete_file("deletable_file.txt");
    helpers.stage_deleted_file("deletable_file.txt");
    helpers.commit();
    let res = repo.diff(usize::MAX);
    assert!(res.is_err());
  }

  // Test case for modifying a file and partially staging the changes
  #[test]
  fn modify_and_partially_stage_file() {
    setup();
    let (helpers, repo) = Git2Helpers::new();
    helpers.create_file("modifiable_file.txt");
    helpers.commit();
    helpers.replace_file("modifiable_file.txt"); // Unstaged changes
    helpers.stage_file("modifiable_file.txt"); // Stage only the initial content
    let (_, files) = repo.diff(usize::MAX).expect("Could not generate diff");
    assert_eq!(files, vec!["modifiable_file.txt"]);
  }

  // Test case for modifying a file and staging all changes
  #[test]
  fn modify_and_stage_all_changes() {
    setup();
    let (helpers, repo) = Git2Helpers::new();
    helpers.create_file("file_to_modify.txt");
    helpers.commit();
    helpers.replace_file("file_to_modify.txt"); // Modify the file
    helpers.stage_file("file_to_modify.txt"); // Stage all changes
    helpers.commit();
    let res = repo.diff(usize::MAX);
    assert!(res.is_err());
  }

  // Test case for handling multiple file operations in a single commit
  #[test]
  fn handle_multiple_file_operations() {
    setup();
    let (helpers, repo) = Git2Helpers::new();
    helpers.create_file("file1.txt");
    helpers.create_file("file2.txt");
    helpers.stage_file("file1.txt");
    helpers.stage_file("file2.txt");
    helpers.commit();

    helpers.replace_file("file1.txt"); // Modify file1
    helpers.delete_file("file2.txt"); // Delete file2
    helpers.stage_file("file1.txt"); // Stage modification of file1
    helpers.stage_deleted_file("file2.txt"); // Stage deletion of file2
    helpers.commit();

    let res = repo.diff(usize::MAX);
    assert!(res.is_err());
  }

  // Test case for unstaged changes after committing
  #[test]
  fn unstaged_changes_after_commit() {
    setup();
    let (helpers, repo) = Git2Helpers::new();
    helpers.create_file("file_to_change.txt");
    helpers.commit();
    helpers.replace_file("file_to_change.txt"); // Modify the file without staging
    let res = repo.diff(usize::MAX);
    assert!(res.is_err());
  }

  // Test case for adding a new file without staging or committing
  #[test]
  fn add_new_file_without_staging() {
    setup();
    let (helpers, repo) = Git2Helpers::new();
    helpers.create_file("new_unstaged_file.txt"); // Create the file without staging
    let res = repo.diff(usize::MAX);
    assert!(res.is_err());
  }
}
