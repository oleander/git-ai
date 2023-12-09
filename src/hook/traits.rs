use std::io::{Read, Write};
use std::path::PathBuf;
use std::fs::File;

#[cfg(not(mock))]
use git2::{DiffFormat, DiffOptions, Repository, Tree};
use anyhow::Result;

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

pub trait PatchRepository {
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

#[cfg(test)]
mod tests {
  use super::*;
  // tempfile
  use tempfile::NamedTempFile;

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
}
