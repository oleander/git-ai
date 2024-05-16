use std::path::{Path, PathBuf};
use std::{env, fs};
use std::os::unix::fs::symlink as symlink_unix;

use colored::Colorize;
use console::Emoji;
use git2::{Repository, RepositoryOpenFlags as Flags};
use anyhow::{bail, Context, Result};
use file::Filesystem;

const EMOJI: Emoji<'_, '_> = Emoji("🔗", "");

// Git hook: prepare-commit-msg
// Crates an executable git hook (prepare-commit-msg) in the .git/hooks directory
pub fn main() -> Result<()> {
  env_logger::init();

  let filesystem = Filesystem::new()?;

  if !filesystem.git_hooks_path().exists() {
    filesystem.git_hooks_path().create_dir_all()?;
  }

  let hook_file = filesystem.prepare_commit_msg_path()?;
  let hook_bin = filesystem.git_ai_hook_bin_path()?;

  if hook_file.exists() {
    hook_file.delete()?;
  }

  hook_file.symlink(hook_bin)?;

  println!(
    "{EMOJI} Hook symlinked successfully to {}",
    hook_file.relative_path()?.to_string().italic()
  );

  Ok(())
}
