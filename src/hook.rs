use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::fs::File;

use structopt::StructOpt;
use git2::{Diff, DiffFormat, DiffOptions, Repository, Tree};
use anyhow::{bail, Context, Result};
use thiserror::Error;

use crate::model::Model;

pub trait FilePath {
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

trait DiffDeltaPath {
  fn path(&self) -> PathBuf;
}

impl DiffDeltaPath for git2::DiffDelta<'_> {
  fn path(&self) -> PathBuf {
    self
      .new_file()
      .path()
      .or_else(|| self.old_file().path())
      .map(PathBuf::from)
      .unwrap_or_default()
  }
}

pub trait Utf8String {
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

pub trait PatchDiff {
  fn to_patch(&self, max_token_count: usize, model: Model) -> Result<String>;
}

impl PatchDiff for Diff<'_> {
  fn to_patch(&self, max_tokens: usize, model: Model) -> Result<String> {
    let mut files: HashMap<PathBuf, String> = HashMap::new();

    self
      .print(DiffFormat::Patch, |diff, _hunk, line| {
        let content = line.content();
        let string = content.to_utf8();

        // Include both changes and context, but prefix context lines with "context: "
        // This helps the model understand the context while still identifying actual changes
        let line_content = match line.origin() {
          '+' | '-' => string,
          _ => format!("context: {}", string)
        };

        match files.get(&diff.path()) {
          Some(file_acc) => {
            files.insert(diff.path(), file_acc.to_owned() + &line_content);
          }
          None => {
            files.insert(diff.path(), line_content);
          }
        }

        true
      })
      .context("Failed to print diff")?;

    let mut diffs: Vec<_> = files.values().collect();

    // TODO: No unwrap
    diffs.sort_by_key(|diff| model.count_tokens(diff).unwrap());

    diffs
      .iter()
      .enumerate()
      .try_fold(
        (max_tokens, String::new(), files.len()),
        |(remaining_tokens, mut final_diff, total_files), (index, diff)| {
          let files_remaining = total_files.saturating_sub(index);
          let max_tokens_per_file = remaining_tokens.saturating_div(files_remaining);

          log::debug!("max_tokens_per_file: {}", max_tokens_per_file);
          log::debug!("remaining_tokens: {}", remaining_tokens);
          log::debug!("total_files: {}", total_files);
          log::debug!("index: {}", index);

          if max_tokens_per_file == 0 {
            bail!("No tokens left to generate commit message. Try increasing the max-tokens configuration option using `git ai config set max-tokens <value>`");
          }

          let file_token_count = model.count_tokens(diff).context("Failed to count diff tokens")?;
          let token_limits = [file_token_count, max_tokens_per_file];
          let file_allocated_tokens = token_limits.iter().min().unwrap();

          // We have reached the token limit for the file: truncate
          let truncated_diff = if file_token_count > *file_allocated_tokens {
            model.truncate(diff, *file_allocated_tokens)
          } else {
            Ok((*diff).clone().to_owned()) // TODO: Better way?
          };

          log::debug!("file_token_count: {}", file_token_count);
          log::debug!("file_allocated_tokens: {}", file_allocated_tokens);
          log::debug!("diff: {}", diff);
          log::debug!("truncated_diff: {:?}", truncated_diff);
          log::debug!("remaining_tokens: {}", remaining_tokens);
          log::debug!("final_diff: {}", final_diff);

          final_diff += &("\n".to_owned() + &truncated_diff.context("Failed to truncate diff")?);

          Ok((remaining_tokens.saturating_sub(*file_allocated_tokens), final_diff, total_files))
        }
      )
      .map(|(_, final_diff, _)| final_diff)
  }
}

pub trait PatchRepository {
  fn to_patch(&self, tree: Option<Tree<'_>>, max_token_count: usize, model: Model) -> Result<String>;
  fn to_diff(&self, tree: Option<Tree<'_>>) -> Result<git2::Diff<'_>>;
}

impl PatchRepository for Repository {
  fn to_patch(&self, tree: Option<Tree>, max_token_count: usize, model: Model) -> Result<String> {
    self.to_diff(tree)?.to_patch(max_token_count, model)
  }

  fn to_diff(&self, tree: Option<Tree<'_>>) -> Result<git2::Diff<'_>> {
    let mut opts = DiffOptions::new();
    opts
      .ignore_whitespace_change(true)
      .recurse_untracked_dirs(false)
      .recurse_ignored_dirs(false)
      .ignore_whitespace_eol(true)
      .ignore_blank_lines(true)
      .include_untracked(false)
      .ignore_whitespace(true)
      .indent_heuristic(false)
      .ignore_submodules(true)
      .include_ignored(false)
      .interhunk_lines(0)
      .context_lines(0)
      .patience(true)
      .minimal(true);

    self
      .diff_tree_to_index(tree.as_ref(), None, Some(&mut opts))
      .context("Failed to get diff")
  }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "commit-msg-hook", about = "A tool for generating commit messages.")]
pub struct Args {
  pub commit_msg_file: PathBuf,

  #[structopt(short = "t", long = "type")]
  pub commit_type: Option<String>,

  #[structopt(short = "s", long = "sha1")]
  pub sha1: Option<String>
}

#[derive(Error, Debug)]
pub enum HookError {
  #[error("Failed to open repository")]
  OpenRepository,

  #[error("Failed to get patch")]
  GetPatch,

  #[error("Empty diff output")]
  EmptyDiffOutput,

  #[error("Failed to write commit message")]
  WriteCommitMessage,

  // anyhow
  #[error(transparent)]
  Anyhow(#[from] anyhow::Error)
}
