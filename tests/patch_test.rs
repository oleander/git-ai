mod common;

use std::path::PathBuf;

use tempfile::NamedTempFile;
use anyhow::Result;
use git2::{DiffFormat, DiffOptions, Repository, Tree};
use anyhow::{Context, Result};
use ai::hook::*;
use common::*;

#[test]
fn test_file_path_is_empty() {
  let named_file = NamedTempFile::new().unwrap();
  let path = named_file.path().to_path_buf();
  assert!(path.is_empty().unwrap());
}

#[test]
fn test_file_path_write_and_read() {
  let named_file = NamedTempFile::new().unwrap();
  let path = named_file.path().to_path_buf();
  let message = "Hello, world!";

  path.write(message.to_string()).unwrap();

  let contents = path.read().unwrap();

  assert_eq!(contents, message);
}

#[test]
fn test_utf8_string_to_utf8() {
  let bytes = vec![72, 101, 108, 108, 111];
  let utf8_string = bytes.to_utf8();

  assert_eq!(utf8_string, "Hello");
}

trait TestPatchDiff {
  fn is_empty(&self) -> Result<bool>;
  fn contains(&self, file: &GitFile) -> Result<bool>;
}

impl TestPatchDiff for git2::Diff<'_> {
  fn is_empty(&self) -> Result<bool> {
    let mut has_changes = false;

    self.foreach(
      &mut |_file, _progress| {
        has_changes = true;
        true
      },
      None,
      None,
      None
    )?;

    Ok(!has_changes)
  }

  fn contains(&self, our_file: &GitFile) -> Result<bool> {
    let mut found = false;
    let our_file_path = our_file.path.strip_prefix(&our_file.repo_path).unwrap();

    self.foreach(
      &mut |file, _progress| {
        let other_path = file.new_file().path().unwrap();
        if other_path == our_file_path {
          found = true;
        }

        true
      },
      None,
      None,
      None
    )?;

    Ok(found)
  }
}

trait TestRepository {
  fn to_diff(&self, tree: Option<Tree<'_>>) -> Result<git2::Diff<'_>>;
}

impl TestRepository for Repository {
  fn to_diff(&self, tree: Option<Tree<'_>>) -> Result<git2::Diff<'_>> {
    let mut opts = DiffOptions::new();
    opts
      .include_untracked(true)
      .recurse_untracked_dirs(true)
      .show_untracked_content(true);

    match tree {
      Some(tree) => {
        // For staged changes, compare tree to index
        let diff = self.diff_tree_to_index(Some(&tree), None, Some(&mut opts))?;
        if !diff.is_empty()? {
          return Ok(diff);
        }
        // If no staged changes, compare tree to workdir
        self.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts))
      }
      None => {
        // For initial state, compare HEAD to workdir
        match self.head() {
          Ok(head) => {
            let tree = head.peel_to_tree()?;
            self.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts))
          }
          Err(_) => {
            // No HEAD yet, show all files as new
            self.diff_tree_to_workdir(None, Some(&mut opts))
          }
        }
      }
    }
    .context("Failed to get diff")
  }
}

#[test]
fn test_patch_diff_to_patch() {
  let repo = TestRepo::default();
  let file = repo.create_file("test.txt", "Hello, world!").unwrap();

  // Get initial diff before staging
  let repo_path = repo.repo_path.path().to_path_buf();
  let git_repo = git2::Repository::open(repo_path).unwrap();
  let diff = TestRepository::to_diff(&git_repo, None).unwrap();
  assert!(!TestPatchDiff::is_empty(&diff).unwrap());

  file.stage().unwrap();
  file.commit().unwrap();

  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
<<<<<<< HEAD

  let diff = git_repo.to_commit_diff(Some(tree.clone())).unwrap();
=======
  let diff = TestRepository::to_diff(&git_repo, Some(tree.clone())).unwrap();
>>>>>>> origin/main
  assert!(TestPatchDiff::is_empty(&diff).unwrap());

  // Add a new line to the file
  let file = repo.create_file("file", "Hello, world!\n").unwrap();
<<<<<<< HEAD
  let diff = git_repo.to_commit_diff(Some(tree.clone())).unwrap();
  assert!(TestPatchDiff::is_empty(&diff).unwrap());
=======
  let diff = TestRepository::to_diff(&git_repo, Some(tree.clone())).unwrap();
  assert!(!TestPatchDiff::is_empty(&diff).unwrap());
  assert!(TestPatchDiff::contains(&diff, &file).unwrap());
>>>>>>> origin/main

  // stage and commit the file
  file.stage().unwrap();
<<<<<<< HEAD
  let diff = git_repo.to_commit_diff(Some(tree.clone())).unwrap();
  assert!(!TestPatchDiff::is_empty(&diff).unwrap());
  assert!(diff.contains(&file).unwrap());

  // commit the file
  file.commit().unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
  let diff = git_repo.to_commit_diff(Some(tree.clone())).unwrap();
  assert!(TestPatchDiff::is_empty(&diff).unwrap());
  assert!(!diff.contains(&file).unwrap());

  // delete the file
  file.delete().unwrap();
  let diff = git_repo.to_commit_diff(Some(tree.clone())).unwrap();
  assert!(TestPatchDiff::is_empty(&diff).unwrap());
=======
  let diff = TestRepository::to_diff(&git_repo, Some(tree.clone())).unwrap();
  assert!(!TestPatchDiff::is_empty(&diff).unwrap());
  file.commit().unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
  let diff = TestRepository::to_diff(&git_repo, Some(tree.clone())).unwrap();
  assert!(TestPatchDiff::is_empty(&diff).unwrap());

  // delete the file
  file.delete().unwrap();
  let diff = TestRepository::to_diff(&git_repo, Some(tree.clone())).unwrap();
  assert!(!TestPatchDiff::is_empty(&diff).unwrap());
  assert!(TestPatchDiff::contains(&diff, &file).unwrap());
>>>>>>> origin/main

  // stage and commit the deletion
  file.stage().unwrap();
<<<<<<< HEAD
  let diff = git_repo.to_commit_diff(Some(tree.clone())).unwrap();
  assert!(!TestPatchDiff::is_empty(&diff).unwrap());
  assert!(diff.contains(&file).unwrap());
=======
  let diff = TestRepository::to_diff(&git_repo, Some(tree.clone())).unwrap();
  assert!(!TestPatchDiff::is_empty(&diff).unwrap());
  assert!(TestPatchDiff::contains(&diff, &file).unwrap());

  file.commit().unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
  let diff = TestRepository::to_diff(&git_repo, Some(tree.clone())).unwrap();
  assert!(TestPatchDiff::is_empty(&diff).unwrap());

  // test initial commit
  let repo = TestRepo::default();
  let file = repo.create_file("test.txt", "Hello, world!").unwrap();
  let repo_path = repo.repo_path.path().to_path_buf();
  let git_repo = git2::Repository::open(repo_path).unwrap();
  let diff = TestRepository::to_diff(&git_repo, None).unwrap();
  assert!(!TestPatchDiff::is_empty(&diff).unwrap());
  assert!(TestPatchDiff::contains(&diff, &file).unwrap());

  // stage and commit the file
  file.stage().unwrap();
  let diff = TestRepository::to_diff(&git_repo, None).unwrap();
  assert!(!TestPatchDiff::is_empty(&diff).unwrap());
  assert!(TestPatchDiff::contains(&diff, &file).unwrap());

  file.commit().unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
  let diff = TestRepository::to_diff(&git_repo, Some(tree.clone())).unwrap();
  assert!(TestPatchDiff::is_empty(&diff).unwrap());
>>>>>>> origin/main
}
