#![feature(assert_matches)]

#[cfg(not(mock))]
// Hook: prepare-commit-msg
use ai::chat::generate_commit_message;
use indicatif::ProgressBar;

use std::process::Termination;
use indicatif::ProgressStyle;
use lazy_static::lazy_static;
use std::process::ExitCode;
use dotenv_codegen::dotenv;
use std::path::PathBuf;
use git2::DiffOptions;
use git2::Repository;
use git2::DiffFormat;
use anyhow::Context;
use std::io::Write;
use anyhow::Result;
use std::io::Read;
use std::fs::File;
use anyhow::bail;
use clap::Parser;
use git2::Tree;
use git2::Oid;

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

#[tokio::main]
async fn main() -> Result<Msg, Box<dyn std::error::Error>> {
  env_logger::init();
  let args = Args::parse();
  Ok(run(args).await?)
}

async fn run(args: Args) -> Result<Msg> {
  let pb = ProgressBar::new_spinner();
  pb.set_style(ProgressStyle::default_spinner()
        .tick_strings(&["-", "\\", "|", "/"])
        .template("{spinner:.blue} Processing {msg}")?);

  if args.commit_type.is_some() {
    return Ok(Msg("Commit message is not empty".to_string()));
  }

  let repo = Repository::open_from_env().context("Failed to open repository")?;

  let tree = if let Some(sha1) = args.sha1 {
    repo.find_commit(sha1).ok().and_then(|commit| commit.tree().ok())
  } else {
    repo.head().ok().and_then(|head| head.peel_to_tree().ok())
  };

  let max_tokens = ai::config::get("max-diff-tokens").unwrap_or(*MAX_DIFF_TOKENS as i32);
  let patch = repo.to_patch(tree, max_tokens.try_into().unwrap()).context("Failed to get patch")?;

  if patch.is_empty() {
    bail!("Empty diff output");
  }

  let new_commit_message = generate_commit_message(patch.to_string()).await?;

  args
    .commit_msg_file
    .write(new_commit_message.trim().to_string())
    .context("Failed to write commit message")?;

  pb.finish_with_message(new_commit_message.clone());
  Ok(Msg(new_commit_message))
}

#[cfg(mock)]
async fn generate_commit_message(diff: String) -> Result<String> {
  Ok(diff.to_string())
}

#[cfg(test)]
mod tests {
  use tempfile::{NamedTempFile, TempDir};
  use std::process::Command as Cmd;
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
