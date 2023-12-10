#![feature(assert_matches)]

use std::path::PathBuf;

use git2::DiffFormat;
use ai::hook::*;
use tempfile::{NamedTempFile, TempDir};
use anyhow::Result;

struct TestRepo {
  repo:      git2::Repository,
  repo_path: TempDir
}

impl Default for TestRepo {
  fn default() -> Self {
    let repo_path = TempDir::new().unwrap();
    let repo = git2::Repository::init(repo_path.path()).unwrap();
    std::env::set_var("GIT_DIR", repo_path.path().join(".git"));

    Self {
      repo,
      repo_path
    }
  }
}

impl TestRepo {
  fn create_file(&self, name: &str, content: &str) -> Result<GitFile> {
    let file_path = self.repo_path.path().join(name);
    std::fs::write(&file_path, content)?;
    let repo = git2::Repository::open(self.repo.path()).unwrap();
    Ok(GitFile::new(repo, file_path, self.repo_path.path().to_path_buf()))
  }
}

struct GitFile {
  repo:      git2::Repository,
  path:      PathBuf,
  repo_path: PathBuf
}

impl GitFile {
  fn new(repo: git2::Repository, path: PathBuf, repo_path: PathBuf) -> Self {
    Self {
      repo,
      path,
      repo_path
    }
  }

  pub fn stage(&self) -> Result<()> {
    let mut index = self.repo.index()?;

    let relative_path = self.path.strip_prefix(&self.repo_path).unwrap();
    if !self.path.exists() {
      index.remove_path(relative_path)?;
      index.write()?;
    } else {
      index.add_path(relative_path)?;
      index.write()?;
    }

    Ok(())
  }

  pub fn commit(&self) -> Result<()> {
    let mut index = self.repo.index()?;
    let oid = index.write_tree()?;
    let signature = git2::Signature::now("Your Name", "email@example.com")?;
    let tree = self.repo.find_tree(oid)?;

    match self.find_last_commit() {
      Ok(parent_commit) => {
        self
          .repo
          .commit(Some("HEAD"), &signature, &signature, "Commit message", &tree, &[&parent_commit])?;
      },
      Err(_) => {
        self.repo.commit(Some("HEAD"), &signature, &signature, "Initial commit", &tree, &[])?;
      }
    }

    Ok(())
  }

  pub fn delete(&self) -> Result<()> {
    std::fs::remove_file(&self.path)?;
    Ok(())
  }

  fn find_last_commit(&self) -> Result<git2::Commit, git2::Error> {
    let head = match self.repo.head() {
      Ok(head) => head,
      Err(e) => {
        if e.code() == git2::ErrorCode::UnbornBranch || e.code() == git2::ErrorCode::NotFound {
          return Err(e);
        } else {
          panic!("Failed to retrieve HEAD: {}", e);
        }
      },
    };

    let commit = head.peel_to_commit()?;
    Ok(commit)
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
