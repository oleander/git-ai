use anyhow::{bail, Result};
use colored::Colorize;
use console::Emoji;
use ai::filesystem::Filesystem;

const EMOJI: Emoji<'_, '_> = Emoji("🔗", "");

pub fn run() -> Result<()> {
  let fs = Filesystem::new()?;

  if !fs.git_hooks_path().exists() {
    fs.git_hooks_path().create_dir_all()?;
  }

  install(&fs)
}

pub fn install(fs: &Filesystem) -> Result<()> {
  let hook_bin = fs.git_ai_hook_bin_path()?;
  let hook_file = fs.prepare_commit_msg_path()?;

  if hook_file.exists() {
    bail!(
      "Hook already exists at {}, please run 'git ai hook reinstall'",
      hook_file.to_string().italic()
    );
  }

  hook_file.symlink(&hook_bin)?;

  println!("{EMOJI} Hook symlinked successfully to {}", hook_file.to_string().italic());

  Ok(())
}
