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

impl FilePath for PathBuf {
  fn write(&self, msg: String) -> Result<()> {
    let mut file = File::create(self)?;
    file.write_all(msg.as_bytes())?;
    Ok(())
  }

  fn read(&self) -> Result<String> {
    let mut file = File::open(self)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
  }
}

lazy_static! {
  // static ref FILE: NamedTempFile = NamedTempFile::new().unwrap();
  // static ref REPO_PATH: TempDir = TempDir::new().unwrap();
}

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

    Self { repo_path }
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
  path: PathBuf,
  repo_path: PathBuf
}

impl GitFile {
  fn new(path: PathBuf, repo_path: PathBuf) -> Result<Self> {
    Ok(Self { path, repo_path })
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

// fn setup_repo() -> Result<TempDir> {
//   let repo_path = TempDir::new().unwrap();

//   let output = Cmd::new("git")
//     .arg("init")
//     .current_dir(repo_path.path())
//     .output()
//     .expect("Failed to execute git init");

//   assert!(output.status.success());

//   std::env::set_var("GIT_DIR", repo_path.path().join(".git"));

//   // // create file
//   // let file_path = repo_path.path().join("file");
//   // let mut file = File::create(file_path.clone())?;
//   // file.write_all(b"Hello, world!")?;

//   // let output = Cmd::new("git")
//   //   .current_dir(repo_path.path())
//   //   .arg("add")
//   //   .arg(file_path)
//   //   .output()
//   //   .expect("Failed to execute git add");

//   // assert!(output.status.success());

//   let output = Cmd::new("git")
//     .arg("commit")
//     .arg("-m")
//     .arg("Initial commit")
//     .current_dir(repo_path.path())
//     .output()
//     .expect("Failed to execute git commit");

//   assert!(output.status.success());

//   Ok(repo_path)
// }

// #[tokio::test]
// async fn test_nothing_to_commit() {
//   let _repo_dir = setup_repo().unwrap();
//   let commit_msg_file = NamedTempFile::new().unwrap();

//   let args = Args {
//     commit_msg_file: commit_msg_file.path().to_path_buf(), commit_type: None, sha1: None
//   };

//   let result = run(&args).await;
//   assert_matches!(result, Err(HookError::EmptyDiffOutput));
//   assert!(commit_msg_file.is_empty().unwrap());
// }

#[tokio::test]
async fn test_something_to_commit() {
  let repository = TestRepo::default();
  let commit_msg_file = NamedTempFile::new().unwrap();

  let args = Args {
    commit_msg_file: commit_msg_file.path().to_path_buf(), commit_type: None, sha1: None
  };

  let result = run(&args).await;
  assert_matches!(result, Err(HookError::EmptyDiffOutput));
  // assert!(!commit_msg_file.is_empty().unwrap());

  // Add a file to the repo
  repository.create_file("file2", "Hello, world!").unwrap();

  let result = run(&args).await;
  assert_matches!(result, Err(HookError::EmptyDiffOutput));
  // assert!(!commit_msg_file.is_empty().unwrap());

  // Add a file to the repo
  let file = repository.create_file("file3", "Hello, world!").unwrap();
  file.stage().unwrap(); // git add file3

  let result = run(&args).await;
  assert_matches!(result, Ok(()));
  // assert!(!commit_msg_file.is_empty().unwrap());

  // Commit file
  file.commit().unwrap(); // git commit -m "Add file3"

  let result = run(&args).await;
  // assert_matches!(result, Err(HookError::EmptyDiffOutput));

  // Delete file
  file.delete().unwrap(); // rm file3

  let result = run(&args).await;
  assert_matches!(result, Err(HookError::EmptyDiffOutput));


  // Add deleted file
  file.stage().unwrap(); // git add file3

  let result = run(&args).await;
  assert_matches!(result, Ok(()));
}

// #[tokio::test]
// async fn test_non_empty_commit_type() {
//   let _repo_dir = setup_repo().unwrap();
//   let commit_msg_file = NamedTempFile::new().unwrap();

//   let args = Args {
//     commit_msg_file: commit_msg_file.path().to_path_buf(),
//     commit_type:     Some("test".to_string()),
//     sha1:            None
//   };

//   let result = run(&args).await;
//   assert_eq!(commit_msg_file.read().unwrap(), "");
//   assert!(result.is_ok());
// }
