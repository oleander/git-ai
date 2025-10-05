//! Utility traits for diff processing.

use std::path::PathBuf;
use std::fs::File;
use std::io::{Read, Write};

use anyhow::Result;

/// Extension trait for PathBuf to support file operations needed for commits
pub trait FilePath {
  fn is_empty(&self) -> Result<bool> {
    self.read().map(|s| s.is_empty())
  }

  fn write(&self, msg: String) -> Result<()>;
  fn read(&self) -> Result<String>;
}

impl FilePath for PathBuf {
  fn write(&self, msg: String) -> Result<()> {
    File::create(self)?
      .write_all(msg.as_bytes())
      .map_err(Into::into)
  }

  fn read(&self) -> Result<String> {
    let mut contents = String::new();
    File::open(self)?.read_to_string(&mut contents)?;
    Ok(contents)
  }
}

/// Extension trait for git2::DiffDelta to get file paths
pub trait DiffDeltaPath {
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

/// Extension trait for converting bytes to UTF-8 strings
pub trait Utf8String {
  fn to_utf8(&self) -> String;
}

impl Utf8String for Vec<u8> {
  fn to_utf8(&self) -> String {
    // Fast path for valid UTF-8 (most common case)
    if let Ok(s) = std::str::from_utf8(self) {
      return s.to_string();
    }
    // Fallback for invalid UTF-8
    String::from_utf8_lossy(self).into_owned()
  }
}

impl Utf8String for [u8] {
  fn to_utf8(&self) -> String {
    // Fast path for valid UTF-8 (most common case)
    if let Ok(s) = std::str::from_utf8(self) {
      return s.to_string();
    }
    // Fallback for invalid UTF-8
    String::from_utf8_lossy(self).into_owned()
  }
}
