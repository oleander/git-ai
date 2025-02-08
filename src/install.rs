use anyhow::{bail, Result};
use colored::Colorize;
use console::Emoji;

use crate::filesystem::Filesystem;

const EMOJI: Emoji<'_, '_> = Emoji("🔗", "");

pub fn install(fs: &Filesystem) -> Result<()> {
  let hook_bin = fs.hook_bin()?;
  let hook_file = fs.hook_file()?;

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
