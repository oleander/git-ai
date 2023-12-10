use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use std::{env, fs};

use git2::{Repository, RepositoryOpenFlags as Flags};
use anyhow::{bail, Context, Result};
use log::debug;

// Git hook: prepare-commit-msg
// Crates an executable git hook (prepare-commit-msg) in the .git/hooks directory
pub fn run() -> Result<()> {
  let curr_bin = env::current_exe().context("Failed to get current executable")?;
  let exec_path = curr_bin.parent().context("Failed to get parent directory")?;
  let hook_bin = exec_path.join("git-ai-hook");

  if !hook_bin.exists() {
    return Err(anyhow::anyhow!("Executable not found: {:?}", hook_bin));
  }
  // absolute path to this executable
  // let script = include_bytes!("../target/debug/git-ai-hook");

  let current_dir = env::current_dir().with_context(|| "Failed to get current directory".to_string())?;
  debug!("Current directory: {:?}", current_dir);

  let repo = Repository::open_ext(&current_dir, Flags::empty(), Vec::<&Path>::new())
    .with_context(|| "Failed to open repository".to_string())?;
  let git_path = repo.path().parent().context("Failed to get repository path")?.join(".git");

  let hook_dir = git_path.join("hooks");
  if !hook_dir.exists() {
    fs::create_dir_all(&hook_dir).context("Failed to create .git/hooks")?;
  }

  let hook_file = hook_dir.join("prepare-commit-msg");
  if hook_file.exists() {
    bail!("Hook file at .git/hooks/prepare-commit-msg already exists, please remove it first using 'git ai hook uninstall'");
  }

  // Symlink the hook_bin to the hook_file
  unix_fs::symlink(&hook_bin, &hook_file).with_context(|| format!("Failed to symlink {:?} to {:?}", hook_bin, hook_file))?;

  println!("Hook symlinked successfully to .git/hooks/prepare-commit-msg");

  Ok(())
}
