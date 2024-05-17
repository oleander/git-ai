use colored::Colorize;
use anyhow::{bail, Result};
use ai::filesystem::Filesystem;
use console::Emoji;

const EMOJI: Emoji<'_, '_> = Emoji("🔗", "");

pub fn run() -> Result<()> {
  let filesystem = Filesystem::new()?;

  if !filesystem.git_hooks_path().exists() {
    filesystem.git_hooks_path().create_dir_all()?;
  }

  let hook_file = filesystem.prepare_commit_msg_path()?;
  let hook_bin = filesystem.git_ai_hook_bin_path()?;

  if hook_file.exists() {
    bail!("Hook already exists at {}, please run 'git ai hook reinstall'", hook_file);
  }

  hook_file.symlink(hook_bin)?;

  println!("{EMOJI} Hook symlinked successfully to {}", hook_file.to_string().italic());

  Ok(())
}
