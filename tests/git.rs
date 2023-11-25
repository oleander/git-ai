use git2::{Commit, IndexAddOption, ObjectType, Repository};
use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use std::io::Write;
use std::fs::File;
use log::info;
use ai::git::Repo;

pub struct Git2Helpers {
  dir: TempDir
}

impl Git2Helpers {
  pub fn new() -> (Self, Repo) {
    let helper = Git2Helpers {
      dir: TempDir::new().expect("Could not create temp dir")
    };

    helper.git(&["init"]);

    let repo = Repo::new_with_path(helper.str_path().to_string()).expect("Could not open repo");

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
    file.write_all(random_content.as_bytes()).expect("Could not write to file");
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
    let output =
      Command::new("git").args(args).env("OVERCOMMIT_DISABLE", "1").current_dir(self.path()).output().context("Could not run git command")?;

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

// Empty repo with a single file, no commits
#[test]
fn empty_repo_with_single_file() {
  setup();
  let (helpers, repo) = Git2Helpers::new();
  helpers.create_file("file.txt");
  helpers.stage_file("file.txt");
  let (_, files) = repo.diff(usize::MAX).expect("Could not generate diff");
  assert_eq!(files, vec!["file.txt"]);


}