use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use std::{env, fs};

use colored::Colorize;
use ai::style::Styled;
use console::Emoji;
use git2::{Repository, RepositoryOpenFlags as Flags};
use anyhow::{Context, Result};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InstallError {
  #[error("Failed to get current directory")]
  CurrentDirectory(#[from] std::io::Error),
  #[error(transparent)]
  Anyhow(#[from] anyhow::Error),
  #[error("Git error: {0}")]
  Git(#[from] git2::Error),
  #[error("Strip prefix error: {0}")]
  StripPrefix(#[from] std::path::StripPrefixError),
  #[error("Hook binary at {0} not found")]
  HookBinNotFound(PathBuf),
  #[error("Git hook already exists at {0}")]
  GitHookExists(PathBuf),

  #[error("Git repository not found at {0}")]
  GitRepoNotFound(PathBuf)
}

const EMOJI: Emoji<'_, '_> = Emoji("🔗", "");


fn can_override_hook() -> bool {
  std::env::args().collect::<Vec<String>>().iter().any(|arg| arg == "-f")
}

// Git hook: prepare-commit-msg
// Crates an executable git hook (prepare-commit-msg) in the .git/hooks directory
pub fn run() -> Result<(), InstallError> {
  let curr_bin = env::current_exe()?;
  let exec_path = curr_bin.parent().context("Failed to get parent directory")?;
  let hook_bin = exec_path.join("git-ai-hook");

  if !hook_bin.exists() {
    return Err(InstallError::HookBinNotFound(hook_bin));
  }

  let current_dir = env::current_dir()?;
  let repo = Repository::open_ext(&current_dir, Flags::empty(), Vec::<&Path>::new())?;
  let repo_path = repo.path().parent().context("Failed to get parent directory")?;
  let git_path = match repo_path.file_name() {
    Some(name) if name == ".git" => repo_path.to_path_buf(),
    Some(_) => repo_path.join(".git"),
    None => return Err(InstallError::GitRepoNotFound(repo_path.to_path_buf()))
  };

  let hook_dir = git_path.join("hooks");
  if !hook_dir.exists() {
    fs::create_dir_all(&hook_dir)?;
  }

  let hook_file = hook_dir.join("prepare-commit-msg");
  if hook_file.exists() && !can_override_hook() {
    return Err(InstallError::GitHookExists(hook_file.relative_path()));
  }

  // Symlink the hook_bin to the hook_file
  unix_fs::symlink(&hook_bin, &hook_file)?;

  println!(
    "{EMOJI} Hook symlinked successfully to {}",
    hook_file.relative_path().display().to_string().italic()
  );

  Ok(())
}
