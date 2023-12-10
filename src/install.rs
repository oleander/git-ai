use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use std::{env, fs};

use git2::{Repository, RepositoryOpenFlags as Flags};
use anyhow::{bail, Context, Result};
use log::debug;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum InstallError {
  #[error("Failed to get current executable")]
  CurrentExecutable,
  #[error("Hook file at .git/hooks/prepare-commit-msg already exists, please remove it first using 'git ai hook uninstall'")]
  HookFileExists,
  #[error("Failed to get current directory")]
  CurrentDirectory(#[from] std::io::Error),
  #[error("Failed to open repository: {0}")]
  OpenRepository(#[from] git2::Error),
  #[error(transparent)]
  Anyhow(#[from] anyhow::Error)
}

// Git hook: prepare-commit-msg
// Crates an executable git hook (prepare-commit-msg) in the .git/hooks directory
pub fn run() -> Result<(), InstallError> {
  let curr_bin = env::current_exe()?;
  let exec_path = curr_bin.parent().context("Failed to get parent directory")?;
  let hook_bin = exec_path.join("git-ai-hook");

  if !hook_bin.exists() {
    return Err(InstallError::CurrentExecutable);
  }

  let current_dir = env::current_dir()?;
  let repo = Repository::open_ext(current_dir, Flags::empty(), Vec::<&Path>::new())?;
  let git_path = repo.path().parent().context("Failed to get repository path")?;

  let hook_dir = git_path.join("hooks");
  if !hook_dir.exists() {
    fs::create_dir_all(&hook_dir)?;
  }

  let hook_file = hook_dir.join("prepare-commit-msg");
  if hook_file.exists() {
    return Err(InstallError::HookFileExists);
  }

  // Symlink the hook_bin to the hook_file
  unix_fs::symlink(&hook_bin, &hook_file)?;

  println!("Hook symlinked successfully to .git/hooks/prepare-commit-msg");

  Ok(())
}
