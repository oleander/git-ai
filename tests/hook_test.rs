#![feature(assert_matches)]

use std::assert_matches::assert_matches;
use std::assert_matches;
use std::fs::File;
use std::io::{Read, Write};
use std::process::Command as Cmd;
use std::path::PathBuf;

use ai::hook::*;
use lazy_static::lazy_static;
use tempfile::{NamedTempFile, TempDir};
use anyhow::Result;

// impl From<NamedTempFile> for PathBuf {
//   fn from(file: NamedTempFile) -> Self {
//     file.path().to_path_buf()
//   }
// }

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
  // static ref FILE: NamedTempFile = NamedTempFile::new().unwrap();
  // static ref REPO_PATH: TempDir = TempDir::new().unwrap();
}

fn setup_repo() -> Result<TempDir> {
  let repo_path = TempDir::new().unwrap();

  let output = Cmd::new("git")
    .arg("init")
    .current_dir(repo_path.path())
    .output()
    .expect("Failed to execute git init");

  assert!(output.status.success());

  // create file
  let file_path = repo_path.path().join("file");
  let mut file = File::create(file_path.clone())?;
  file.write_all(b"Hello, world!")?;

  let output = Cmd::new("git")
    .current_dir(repo_path.path())
    .arg("add")
    .arg(file_path)
    .output()
    .expect("Failed to execute git add");

  assert!(output.status.success());
  // list and print all files & dir in the repo not using git
  // let output = Cmd::new("ls")
  //   .current_dir(repo_path.path())
  //   .arg("-la")
  //   .output()
  //   .expect("Failed to execute ls");

  // println!("ls output: {}", String::from_utf8(output.stdout).unwrap());

  // Set the env GIT_DIR to the repository path
  // let git_dir = repo_path.path().join(".git");
  std::env::set_var("GIT_DIR", repo_path.path().join(".git"));

  assert!(output.status.success());

  let output = Cmd::new("git")
    .arg("commit")
    .arg("-m")
    .arg("Initial commit")
    .current_dir(repo_path.path())
    .output()
    .expect("Failed to execute git commit");

  assert!(output.status.success());

  Ok(repo_path)
}

#[tokio::test]
async fn test_nothing_to_commit() {
  let _repo_dir = setup_repo().unwrap();
  let commit_msg_file = NamedTempFile::new().unwrap();

  let args = Args {
    commit_msg_file: commit_msg_file.path().to_path_buf(), commit_type: None, sha1: None
  };

  let result = run(args).await;
  assert_matches!(result, Err(HookError::EmptyDiffOutput));
  assert!(commit_msg_file.is_empty().unwrap());
}

#[tokio::test]
async fn test_non_empty_commit_type() {
  let _repo_dir = setup_repo().unwrap();
  let commit_msg_file = NamedTempFile::new().unwrap();

  let args = Args {
    commit_msg_file: commit_msg_file.path().to_path_buf(), commit_type: Some("test".to_string()), sha1: None
  };

  let result = run(args).await;
  assert_eq!(commit_msg_file.read().unwrap(), "");
  assert!(result.is_ok());
}
