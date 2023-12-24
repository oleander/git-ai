use std::path::{Path, PathBuf};
use std::{env, fs};

use colored::Colorize;
use ai::style::Styled;
use console::Emoji;
use git2::{Repository, RepositoryOpenFlags as Flags};
use anyhow::{bail, Context, Result};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InstallError {
  #[error("Failed to get current directory")]
  CurrentDir,
  #[error("Failed to open repository")]
  OpenRepo,
  #[error("Hook already exists: {0:?}")]
  HookExists(PathBuf)
}

const EMOJI: Emoji<'_, '_> = Emoji("🔗", "");

pub fn run() -> Result<()> {
  let current_dir = env::current_dir().context(InstallError::CurrentDir)?;
  let repo = Repository::open_ext(current_dir, Flags::empty(), Vec::<&Path>::new()).context(InstallError::OpenRepo)?;

  let hook_dir = PathBuf::from(repo.path()).join("hooks");
  let hook_file = hook_dir.join("prepare-commit-msg");

  if !hook_file.exists() {
    bail!(InstallError::HookExists(hook_file));
  }

  fs::remove_file(&hook_file).context("Failed to remove hook file")?;

  println!("{EMOJI} Hook uninstall successfully from {}", hook_file.relative_path().display().to_string().italic());

  Ok(())
}
