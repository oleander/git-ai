use std::path::{Path, PathBuf};
use std::{env, fs};
use std::os::unix::fs::symlink as symlink_unix;

use anyhow::{bail, Context, Result};
use git2::{Repository, RepositoryOpenFlags as Flags};

#[derive(Debug, Clone)]
pub struct Filesystem {
  git_ai_hook_bin_path: PathBuf,
  git_hooks_path:       PathBuf
}

#[derive(Debug, Clone)]
pub struct File {
  path: PathBuf
}

impl File {
  pub fn new(path: PathBuf) -> Self {
    Self { path }
  }

  pub fn exists(&self) -> bool {
    self.path.exists()
  }

  pub fn delete(&self) -> Result<()> {
    log::debug!("Removing file at {}", self);
    fs::remove_file(&self.path).context(format!("Failed to remove file at {}", self))
  }

  pub fn symlink(&self, target: File) -> Result<()> {
    log::debug!("Symlinking {} to {}", target, self);
    symlink_unix(&target.path, &self.path).context(format!("Failed to symlink {} to {}", target, self))
  }

  pub fn relative_path(&self) -> Result<Dir> {
    Dir::new(
      self
        .path
        .strip_prefix(env::current_dir().context("Failed to get current directory")?)
        .context(format!("Failed to strip prefix from {}", self.path.display()))?
        .to_path_buf()
    )
    .into()
  }
}

// implement the trait for rendering using format!("{}", file)
impl std::fmt::Display for File {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.relative_path().unwrap().path.display())
  }
}

impl Into<Result<File>> for File {
  fn into(self) -> Result<File> {
    Ok(self)
  }
}

impl Into<Result<Dir>> for Dir {
  fn into(self) -> Result<Dir> {
    Ok(self)
  }
}

#[derive(Debug, Clone)]
pub struct Dir {
  path: PathBuf
}

impl std::fmt::Display for Dir {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.relative_path().unwrap().path.display())
  }
}

impl Into<Result<Filesystem>> for Filesystem {
  fn into(self) -> Result<Filesystem> {
    Ok(self)
  }
}

impl Dir {
  pub fn new(path: PathBuf) -> Self {
    Self { path }
  }

  pub fn exists(&self) -> bool {
    self.path.exists()
  }

  pub fn create_dir_all(&self) -> Result<()> {
    log::debug!("Creating directory at {}", self);
    fs::create_dir_all(&self.path).context(format!("Failed to create directory at {}", self))
  }

  pub fn relative_path(&self) -> Result<Self> {
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
  fn new() -> Result<Self> {
    let current_dir = env::current_dir().context("Failed to get current directory")?;
    let git_ai_bin_path = env::current_exe().context("Failed to get current executable")?;

    let repo = Repository::open_ext(current_dir.clone(), Flags::empty(), Vec::<&Path>::new())
      .context(format!("Failed to open repository at {}", current_dir.clone().display()))?;

    let git_path = repo.path();

    let git_ai_hook_bin_path = git_ai_bin_path
      .parent()
      .context(format!("Failed to get parent directory of {}", git_ai_bin_path.display()))?
      .join("git-ai-hook");

    if !git_ai_hook_bin_path.exists() {
      bail!("Hook binary not found at {}", git_ai_hook_bin_path.display());
    }

    Self {
      git_ai_hook_bin_path,
      git_hooks_path: git_path.join("hooks")
    }
    .into()
  }

  pub fn git_ai_hook_bin_path(&self) -> Result<File> {
    // Ok(self.git_ai_hook_bin_path.as_path())
    File::new(self.git_ai_hook_bin_path.clone()).into()
  }

  pub fn git_hooks_path(&self) -> Dir {
    Dir::new(self.git_hooks_path.clone()).into()
  }

  pub fn prepare_commit_msg_path(&self) -> Result<File> {
    if !self.git_hooks_path.exists() {
      bail!("Hooks directory not found at {}", self.git_hooks_path.display());
    }

    File::new(self.git_hooks_path.join("prepare-commit-msg")).into()
  }
}
