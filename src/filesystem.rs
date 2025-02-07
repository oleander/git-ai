use std::path::{Path, PathBuf};
use std::{env, fs};
use std::os::unix::fs::symlink as symlink_unix;

use anyhow::{bail, Context, Result};
use git2::{Repository, RepositoryOpenFlags as Flags};

use crate::profile;

/// Represents the filesystem structure for git-ai.
/// Handles paths for hooks and binaries.
#[derive(Debug, Clone)]
pub struct Filesystem {
  git_ai_hook_bin_path: PathBuf,
  git_hooks_path:       PathBuf
}

/// Represents a file in the filesystem.
/// Provides operations for file manipulation.
#[derive(Debug, Clone)]
pub struct File {
  path: PathBuf
}

impl File {
  /// Creates a new File instance.
  ///
  /// # Arguments
  /// * `path` - The path to the file
  pub fn new(path: PathBuf) -> Self {
    Self { path }
  }

  /// Checks if the file exists.
  ///
  /// # Returns
  /// * `bool` - true if the file exists, false otherwise
  pub fn exists(&self) -> bool {
    profile!("Check file exists");
    self.path.exists()
  }

  /// Deletes the file from the filesystem.
  ///
  /// # Returns
  /// * `Result<()>` - Success or an error if deletion fails
  pub fn delete(&self) -> Result<()> {
    profile!("Delete file");
    log::debug!("Removing file at {}", self);
    fs::remove_file(&self.path).context(format!("Failed to remove file at {}", self))
  }

  /// Creates a symbolic link to the target file.
  ///
  /// # Arguments
  /// * `target` - The file to link to
  ///
  /// # Returns
  /// * `Result<()>` - Success or an error if link creation fails
  pub fn symlink(&self, target: File) -> Result<()> {
    profile!("Create symlink");
    log::debug!("Symlinking {} to {}", target, self);
    symlink_unix(&target.path, &self.path).context(format!("Failed to symlink {} to {}", target, self))
  }

  /// Gets the relative path from the current directory.
  ///
  /// # Returns
  /// * `Result<Dir>` - The relative path as a Dir or an error
  pub fn relative_path(&self) -> Result<Dir> {
    profile!("Get relative file path");
    Dir::new(
      self
        .path
        .strip_prefix(env::current_dir().context("Failed to get current directory")?)
        .context(format!("Failed to strip prefix from {}", self.path.display()))?
        .to_path_buf()
    )
    .into()
  }

  /// Gets the parent directory of the file.
  ///
  /// # Returns
  /// * `Dir` - The parent directory
  pub fn parent(&self) -> Dir {
    profile!("Get parent directory");
    Dir::new(self.path.parent().unwrap_or(Path::new("")).to_path_buf())
  }
}

impl From<&File> for Dir {
  fn from(file: &File) -> Self {
    file.parent()
  }
}

impl std::fmt::Display for File {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.relative_path().unwrap_or(self.into()).path.display())
  }
}

impl From<File> for Result<File> {
  fn from(file: File) -> Result<File> {
    Ok(file)
  }
}

/// Represents a directory in the filesystem.
/// Provides operations for directory manipulation.
#[derive(Debug, Clone)]
pub struct Dir {
  path: PathBuf
}

impl std::fmt::Display for Dir {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.path.display())
  }
}

impl From<Dir> for Result<Dir> {
  fn from(dir: Dir) -> Result<Dir> {
    Ok(dir)
  }
}

impl From<Filesystem> for Result<Filesystem> {
  fn from(filesystem: Filesystem) -> Result<Filesystem> {
    Ok(filesystem)
  }
}

impl Dir {
  /// Creates a new Dir instance.
  ///
  /// # Arguments
  /// * `path` - The path to the directory
  pub fn new(path: PathBuf) -> Self {
    Self { path }
  }

  /// Checks if the directory exists.
  ///
  /// # Returns
  /// * `bool` - true if the directory exists, false otherwise
  pub fn exists(&self) -> bool {
    profile!("Check directory exists");
    self.path.exists()
  }

  /// Creates the directory and all parent directories if they don't exist.
  ///
  /// # Returns
  /// * `Result<()>` - Success or an error if creation fails
  pub fn create_dir_all(&self) -> Result<()> {
    profile!("Create directory recursively");
    log::debug!("Creating directory at {}", self);
    fs::create_dir_all(&self.path).context(format!("Failed to create directory at {}", self))
  }

  /// Gets the relative path from the current directory.
  ///
  /// # Returns
  /// * `Result<Self>` - The relative path or an error
  pub fn relative_path(&self) -> Result<Self> {
    profile!("Get relative directory path");
    Self::new(
      self
        .path
        .strip_prefix(env::current_dir().context("Failed to get current directory")?)
        .context(format!("Failed to strip prefix from {}", self.path.display()))?
        .to_path_buf()
    )
    .into()
  }
}

impl Filesystem {
  /// Creates a new Filesystem instance.
  /// Initializes paths for git hooks and binaries.
  ///
  /// # Returns
  /// * `Result<Self>` - The initialized filesystem or an error
  pub fn new() -> Result<Self> {
    profile!("Initialize filesystem");

    let current_dir = {
      profile!("Get current directory");
      env::current_dir().context("Failed to get current directory")?
    };

    let git_ai_bin_path = {
      profile!("Get executable path");
      env::current_exe().context("Failed to get current executable")?
    };

    let repo = {
      profile!("Open git repository");
      Repository::open_ext(current_dir.clone(), Flags::empty(), Vec::<&Path>::new())
        .context(format!("Failed to open repository at {}", current_dir.clone().display()))?
    };

    let mut git_path = repo.path().to_path_buf();
    if git_path.is_relative() {
      profile!("Convert relative git path to absolute");
      git_path = current_dir.join(git_path);
    }

    let git_ai_hook_bin_path = {
      profile!("Get hook binary path");
      git_ai_bin_path
        .parent()
        .context(format!("Failed to get parent directory of {}", git_ai_bin_path.display()))?
        .join("git-ai-hook")
    };

    if !git_ai_hook_bin_path.exists() {
      bail!("Hook binary not found at {}", git_ai_hook_bin_path.display());
    }

    Self {
      git_ai_hook_bin_path,
      git_hooks_path: git_path.join("hooks")
    }
    .into()
  }

  /// Gets the path to the git-ai hook binary.
  ///
  /// # Returns
  /// * `Result<File>` - The hook binary path or an error
  pub fn git_ai_hook_bin_path(&self) -> Result<File> {
    profile!("Get hook binary file");
    File::new(self.git_ai_hook_bin_path.clone()).into()
  }

  /// Gets the path to the git hooks directory.
  ///
  /// # Returns
  /// * `Dir` - The hooks directory path
  pub fn git_hooks_path(&self) -> Dir {
    profile!("Get hooks directory");
    Dir::new(self.git_hooks_path.clone())
  }

  /// Gets the path to the prepare-commit-msg hook.
  ///
  /// # Returns
  /// * `Result<File>` - The hook path or an error
  pub fn prepare_commit_msg_path(&self) -> Result<File> {
    profile!("Get prepare-commit-msg hook path");
    if !self.git_hooks_path.exists() {
      bail!("Hooks directory not found at {}", self.git_hooks_path.display());
    }

    File::new(self.git_hooks_path.join("prepare-commit-msg")).into()
  }
}
