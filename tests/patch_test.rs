#![feature(assert_matches)]

use std::assert_matches::assert_matches;
use std::fs::File;
use std::io::{Read, Write};
use std::process::Command as Cmd;
use std::path::PathBuf;

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
