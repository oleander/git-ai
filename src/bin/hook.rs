#![feature(assert_matches)]

// Hook: prepare-commit-msg

use log::info;
// use ai::chat::generate_commit_message;
use std::process::Termination;
use lazy_static::lazy_static;
use std::process::ExitCode;
use dotenv_codegen::dotenv;
use std::path::PathBuf;
use git2::DiffOptions;
use git2::Repository;
use git2::DiffFormat;
use std::io::Write;
use std::io::Read;
use anyhow::Result;
use std::fs::File;
use clap::Parser;
use git2::Tree;
use git2::Oid;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
  // #[clap(parse(from_os_str))]
  commit_msg_file: PathBuf,

  #[clap(required = false)]
  commit_type: Option<String>,

  #[clap(required = false)]
  sha1: Option<String>
}

lazy_static! {
  static ref MAX_CHARS: usize = dotenv!("MAX_CHARS").parse::<usize>().unwrap();
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

impl Utf8String for [u8] {
  fn to_utf8(&self) -> String {
    String::from_utf8(self.to_vec()).unwrap_or_default()
  }
}

#[tokio::main]
async fn main() -> Result<Msg, Box<dyn std::error::Error>> {
  let args = Args::parse();
  run(args).await?;
  Ok(Msg("Commit message generated".to_string()))
}

async fn run(args: Args) -> Result<Msg, Box<dyn std::error::Error>> {
  if !args.commit_msg_file.is_empty()? {
    return Ok(Msg("Commit message is not empty".to_string()));
  }

  let repo = Repository::open_from_env()?;

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

  let mut length = 0;

  let tree: Option<Tree<'_>> = if let Some(sha1) = args.sha1 {
    repo.find_commit(Oid::from_str(&sha1)?)?.tree()?.into()
  } else {
    repo.head().ok().and_then(|head| head.peel_to_tree().ok())
  };

  let diff = repo.diff_tree_to_index(tree.as_ref(), None, Some(&mut opts))?;
  let mut diff_str = Vec::new();
  let max_token_count = *MAX_CHARS;

  #[rustfmt::skip]
  diff.print(DiffFormat::Patch, |_, _, line| {
    let content = line.content();
    diff_str.extend_from_slice(content);
    let str = content.to_utf8();
    length += str.len();
    length <= max_token_count
  }).ok();

  info!("diff_str: {:?}", diff_str.to_utf8());
  let diff = diff_str.to_utf8();
  let new_commit_message = generate_commit_message(diff).await?;

  args.commit_msg_file.write(new_commit_message)?;

  Ok(Msg("Commit message generated".to_string()))
}

async fn generate_commit_message(_: String) -> Result<String> {
  Ok("Commit message generated".to_string())
}

#[cfg(test)]
mod tests {
  use tempfile::NamedTempFile;
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

  #[tokio::test]
  async fn test_generate_commit_message() {
    let temp_file = NamedTempFile::new().unwrap();

    let args = Args {
      commit_msg_file: temp_file.path().into(), commit_type: None, sha1: None
    };

    assert!(temp_file.is_empty().unwrap());
    let result = run(args).await;
    assert!(!temp_file.is_empty().unwrap());
    assert!(result.is_ok());
  }
}
