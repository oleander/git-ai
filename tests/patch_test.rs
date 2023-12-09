#![feature(assert_matches)]

use std::assert_matches::assert_matches;
use std::fs::File;
use std::io::{Read, Write};
use std::process::Command as Cmd;
use std::path::PathBuf;

use git2::DiffFormat;
use ai::hook::traits::*;
use tempfile::{NamedTempFile, TempDir};
use anyhow::Result;

struct TestRepo {
  repo_path: TempDir
}

impl Default for TestRepo {
  fn default() -> Self {
    let repo_path = TempDir::new().unwrap();

    let output = Cmd::new("git")
      .arg("init")
      .current_dir(repo_path.path())
      .output()
      .expect("Failed to execute git init");

    assert!(output.status.success());

    std::env::set_var("GIT_DIR", repo_path.path().join(".git"));

    Self {
      repo_path
    }
  }
}

impl TestRepo {
  fn create_file(&self, name: &str, content: &str) -> Result<GitFile> {
    let file_path = self.repo_path.path().join(name);
    file_path.write(content.to_string())?;
    GitFile::new(file_path, self.repo_path.path().to_path_buf())
  }
}

struct GitFile {
  path:      PathBuf,
  repo_path: PathBuf
}

impl GitFile {
  fn new(path: PathBuf, repo_path: PathBuf) -> Result<Self> {
    Ok(Self {
      path,
      repo_path
    })
  }

  pub fn stage(&self) -> Result<()> {
    let output = Cmd::new("git")
      .arg("add")
      .arg(&self.path)
      .current_dir(&self.repo_path)
      .output()
      .expect("Failed to execute git add");

    assert!(output.status.success());

    Ok(())
  }

  pub fn commit(&self) -> Result<()> {
    let output = Cmd::new("git")
      .arg("commit")
      .arg("-m")
      .arg("Initial commit")
      .current_dir(&self.repo_path)
      .output()
      .expect("Failed to execute git commit");

    assert!(output.status.success());

    Ok(())
  }

  pub fn delete(&self) -> Result<()> {
    std::fs::remove_file(&self.path)?;
    Ok(())
  }
}

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
        if (other_path == our_file_name) {
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
