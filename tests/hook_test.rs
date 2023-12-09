#![feature(assert_matches)]

use std::assert_matches::assert_matches;

use std::assert_matches;
use std::fs::File;
use std::io::{Read, Write};
use std::process::Command as Cmd;

use ai::hook::*;
use lazy_static::lazy_static;
use tempfile::{NamedTempFile, TempDir};
use anyhow::Result;

impl FilePath for NamedTempFile {
  fn write(&self, msg: String) -> Result<()> {
    let mut file = File::create(self.path())?;
    file.write_all(msg.as_bytes())?;
    Ok(())
  }

  fn read(&self) -> Result<String> {
    let mut file = File::open(self.path())?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
  }
}

trait FilePath {
  fn is_empty(&self) -> Result<bool> {
    self.read().map(|s| s.is_empty())
  }

  fn write(&self, msg: String) -> Result<()>;
  fn read(&self) -> Result<String>;
}

lazy_static! {
  static ref FILE: NamedTempFile = NamedTempFile::new().unwrap();
  static ref REPO_PATH: TempDir = TempDir::new().unwrap();
}

fn setup_repo() -> Result<()> {
  let output = Cmd::new("git")
    .arg("init")
    .current_dir(REPO_PATH.path())
    .output()
    .expect("Failed to execute git init");

  assert!(output.status.success());

  // create file
  let file_path = REPO_PATH.path().join("file");
  let mut file = File::create(file_path.clone())?;
  file.write_all(b"Hello, world!")?;

  let output = Cmd::new("git")
    .current_dir(REPO_PATH.path())
    .arg("add")
    .arg(file_path)
    .output()
    .expect("Failed to execute git add");

  assert!(output.status.success());

  let output = Cmd::new("git")
    .arg("commit")
    .arg("-m")
    .arg("Initial commit")
    .current_dir(REPO_PATH.path())
    .output()
    .expect("Failed to execute git commit");

  assert!(output.status.success());

  Ok(())
}

#[tokio::test]
async fn test_empty_commit_type() {
  setup_repo().unwrap();

  let args = Args {
    commit_msg_file: FILE.path().into(), commit_type: None, sha1: None
  };

  let result = run(args).await;
  assert_matches!(result, Ok(()));
  assert!(!FILE.is_empty().unwrap());
}

#[tokio::test]
async fn test_non_empty_commit_type() {
  setup_repo().unwrap();

  let args = Args {
    commit_msg_file: FILE.path().into(), commit_type: Some("test".to_string()), sha1: None
  };

  let result = run(args).await;
  assert_eq!(FILE.read().unwrap(), "");
  assert!(result.is_ok());
}
