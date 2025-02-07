mod common;

use std::path::PathBuf;

use tempfile::NamedTempFile;
use git2::DiffFormat;
use anyhow::Result;
use ai::hook::*;
use ai::model::Model;
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
    let mut acc = Vec::new();
    let mut length = 0;

    #[rustfmt::skip]
    self.print(DiffFormat::Patch, |_, _, line| {
      let content = line.content();
      acc.extend_from_slice(content);
      length += content.len();
      true
    })?;

    Ok(length == 0)
  }

  fn contains(&self, our_file: &GitFile) -> Result<bool> {
    let mut found = false;
    let our_file_name = our_file.path.file_name().unwrap();

    self.foreach(
      &mut |file, _progress| {
        let other_path: PathBuf = file.new_file().path().unwrap().to_path_buf();
        if other_path == our_file_name {
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

#[test]
fn test_patch_diff_to_patch() {
  let repo = TestRepo::default();
  let file = repo.create_file("test.txt", "Hello, world!").unwrap();
  file.stage().unwrap();
  file.commit().unwrap();

  let repo_path = repo.repo_path.path().to_path_buf();
  let git_repo = git2::Repository::open(repo_path).unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();

  let diff = git_repo.to_diff(Some(tree.clone())).unwrap();
  assert!(diff.is_empty().unwrap());

  // Add a new line to the file
  let file = repo.create_file("file", "Hello, world!\n").unwrap();
  let diff = git_repo.to_diff(Some(tree.clone())).unwrap();
  assert!(diff.is_empty().unwrap());

  // stage the file
  file.stage().unwrap();
  let diff = git_repo.to_diff(Some(tree.clone())).unwrap();
  assert!(!diff.is_empty().unwrap());
  assert!(diff.contains(&file).unwrap());

  // commit the file
  file.commit().unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
  let diff = git_repo.to_diff(Some(tree.clone())).unwrap();
  assert!(diff.is_empty().unwrap());
  assert!(!diff.contains(&file).unwrap());

  // delete the file
  file.delete().unwrap();
  let diff = git_repo.to_diff(Some(tree.clone())).unwrap();
  assert!(diff.is_empty().unwrap());

  // stage the file
  file.stage().unwrap();
  let diff = git_repo.to_diff(Some(tree.clone())).unwrap();
  assert!(!diff.is_empty().unwrap());
  assert!(diff.contains(&file).unwrap());
}

#[test]
fn test_diff_with_token_limits() {
  let repo = TestRepo::default();
  let file = repo
    .create_file("test.txt", "Initial content\nwith multiple\nlines of text")
    .unwrap();
  file.stage().unwrap();
  file.commit().unwrap();

  // Create a large change that would exceed a small token limit
  let large_content = "New content\n".repeat(100);
  let file = repo.create_file("test.txt", &large_content).unwrap();
  file.stage().unwrap();

  let git_repo = git2::Repository::open(repo.repo_path.path()).unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
  let diff = git_repo.to_diff(Some(tree)).unwrap();

  // Test with a small token limit
  let small_patch = diff.to_patch(10, Model::GPT4oMini).unwrap();
  let large_patch = diff.to_patch(1000, Model::GPT4oMini).unwrap();

  // The small patch should be shorter than the large patch
  assert!(small_patch.len() < large_patch.len());

  // Both patches should contain some content
  assert!(!small_patch.is_empty());
  assert!(!large_patch.is_empty());
}

#[test]
fn test_diff_multiple_files() {
  let repo = TestRepo::default();

  // Create and commit initial files
  let file1 = repo.create_file("file1.txt", "Initial content 1").unwrap();
  let file2 = repo.create_file("file2.txt", "Initial content 2").unwrap();
  file1.stage().unwrap();
  file2.stage().unwrap();
  file1.commit().unwrap();

  // Modify both files
  let file1 = repo
    .create_file("file1.txt", "Modified content 1\nwith more lines")
    .unwrap();
  let file2 = repo
    .create_file("file2.txt", "Modified content 2\nwith more lines")
    .unwrap();
  file1.stage().unwrap();
  file2.stage().unwrap();

  let git_repo = git2::Repository::open(repo.repo_path.path()).unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
  let diff = git_repo.to_diff(Some(tree)).unwrap();

  // Test that both files are included in the patch
  let patch = diff.to_patch(1000, Model::GPT4oMini).unwrap();
  assert!(patch.contains("file1.txt"));
  assert!(patch.contains("file2.txt"));
}

#[test]
fn test_diff_whitespace_handling() {
  let repo = TestRepo::default();
  let file = repo
    .create_file("test.txt", "Line 1\nLine 2\nLine 3")
    .unwrap();
  file.stage().unwrap();
  file.commit().unwrap();

  // Modify with different whitespace
  let file = repo
    .create_file("test.txt", "Line 1  \nLine   2\nLine 3\n")
    .unwrap();
  file.stage().unwrap();

  let git_repo = git2::Repository::open(repo.repo_path.path()).unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
  let diff = git_repo.to_diff(Some(tree)).unwrap();

  // The diff should be minimal due to whitespace handling
  let patch = diff.to_patch(1000, Model::GPT4oMini).unwrap();
  assert!(!patch.contains("Line 1")); // Should ignore whitespace changes
}
