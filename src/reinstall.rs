use console::Emoji;
use anyhow::Result;
use ai::filesystem::Filesystem;
use colored::*;

const EMOJI: Emoji<'_, '_> = Emoji("🔗", "");

pub fn run() -> Result<()> {
  let fs = Filesystem::new()?;
  let hook_bin = fs.git_ai_hook_bin_path()?;
  let hook_file = fs.prepare_commit_msg_path()?;

  if hook_file.exists() {
    log::debug!("Removing existing hook file: {}", hook_file);
    hook_file.delete()?;
  }

  hook_file.symlink(&hook_bin)?;

  println!(
    "{EMOJI} Hook symlinked successfully to {}",
    hook_file.relative_path()?.to_string().italic()
  );

  Ok(())
}
