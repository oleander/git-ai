// Hook: prepare-commit-msg

#![feature(assert_matches)]

use std::process::{ExitCode, Termination};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::fs::File;
use std::time::Duration;

#[cfg(not(mock))]
use git2::{DiffFormat, DiffOptions, Oid, Repository, Tree};
use indicatif::{ProgressBar, ProgressStyle};
use anyhow::{bail, Context, Result};
use lazy_static::lazy_static;
use dotenv_codegen::dotenv;
use ai::chat::generate_commit;
use clap::Parser;
use tokio::time::sleep;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
  commit_msg_file: PathBuf,

  #[clap(required = false)]
  commit_type: Option<String>,

  #[clap(required = false)]
  sha1: Option<Oid>
}

lazy_static! {
  static ref MAX_DIFF_TOKENS: usize = dotenv!("MAX_DIFF_TOKENS").parse::<usize>().unwrap();
}

#[derive(Debug)]
struct Msg(String);

impl Termination for Msg {
  fn report(self) -> ExitCode {
    println!("{}", self.0);
    ExitCode::SUCCESS
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

trait Utf8String {
  fn to_utf8(&self) -> String;
}

impl Utf8String for Vec<u8> {
  fn to_utf8(&self) -> String {
    String::from_utf8(self.to_vec()).unwrap_or_default()
  }
}

impl Utf8String for [u8] {
  fn to_utf8(&self) -> String {
    String::from_utf8(self.to_vec()).unwrap_or_default()
  }
}

trait PatchDiff {
  fn to_patch(&self, max_token_count: usize) -> Result<String>;
}

impl PatchDiff for git2::Diff<'_> {
  fn to_patch(&self, max_token_count: usize) -> Result<String> {
    let mut acc = Vec::new();
    let mut length = 0;

    #[rustfmt::skip]
    self.print(DiffFormat::Patch, |_, _, line| {
      let content = line.content();
      acc.extend_from_slice(content);
      let str = content.to_utf8();
      length += str.len();
      length <= max_token_count
    }).ok();

    Ok(acc.to_utf8())
  }
}

trait PatchRepository {
  fn to_patch(&self, tree: Option<Tree<'_>>, max_token_count: usize) -> Result<String>;
}

impl PatchRepository for Repository {
  fn to_patch(&self, tree: Option<Tree<'_>>, max_token_count: usize) -> Result<String> {
    let mut opts = DiffOptions::new();
    opts
      .enable_fast_untracked_dirs(true)
      .ignore_whitespace_change(true)
      .recurse_untracked_dirs(false)
      .recurse_ignored_dirs(false)
      .ignore_whitespace_eol(true)
      .ignore_blank_lines(true)
      .include_untracked(false)
      .indent_heuristic(false)
      .ignore_submodules(true)
      .include_ignored(false)
      .interhunk_lines(0)
      .context_lines(0)
      .patience(true)
      .minimal(true);

    self.diff_tree_to_index(tree.as_ref(), None, Some(&mut opts))?.to_patch(max_token_count)
  }
}

#[cfg(mock)]
async fn generate_commit_message(diff: String) -> Result<String> {
  Ok(diff.to_string())
}

#[cfg(test)]
mod tests {
  use std::process::Command as Cmd;

  use tempfile::{NamedTempFile, TempDir};

  use super::*;

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
    assert!(result.is_ok());
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
}

async fn run(args: Args) -> Result<()> {
  // If defined, then the user already provided a commit message
  if args.commit_type.is_some() {
    return Ok(());
  }

  // Loading bar to indicate that the program is running
  let pb = ProgressBar::new_spinner();
  pb.set_style(
    ProgressStyle::default_spinner()
      .tick_strings(&["-", "\\", "|", "/"])
      .template("{spinner:.blue} {msg}")?
  );

  pb.set_message("Generating commit message...");

  let pb_clone = pb.clone();
  tokio::spawn(async move {
    loop {
      pb_clone.tick();
      sleep(Duration::from_millis(150)).await;
    }
  });

  let repo = Repository::open_from_env().context("Failed to open repository")?;

  // Get the tree from the commit if the sha1 is provided
  // The sha1 is provided when the user is amending a commit
  let tree = if let Some(sha1) = args.sha1 {
    repo.find_commit(sha1).ok().and_then(|commit| commit.tree().ok())
  } else {
    repo.head().ok().and_then(|head| head.peel_to_tree().ok())
  };

  let max_tokens = ai::config::get("max-diff-tokens").unwrap_or(*MAX_DIFF_TOKENS);
  let patch = repo.to_patch(tree, max_tokens).context("Failed to get patch")?;

  if patch.is_empty() {
    bail!("Empty diff output");
  }

  let commit_message = generate_commit(patch.to_string()).await?;

  args
    .commit_msg_file
    .write(commit_message.trim().to_string())
    .context("Failed to write commit message")?;

  // // Stop the loading bar
  // is_done.store(true, Ordering::SeqCst);

  pb.finish_and_clear();
  Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();
  run(Args::parse()).await
}
