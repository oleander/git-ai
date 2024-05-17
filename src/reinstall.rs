use console::Emoji;
use anyhow::Result;
use ai::filesystem::Filesystem;
use colored::*;

const EMOJI: Emoji<'_, '_> = Emoji("🔗", "");

pub fn run() -> Result<()> {
  env_logger::init();

  let filesystem = Filesystem::new()?;

  if !filesystem.git_hooks_path().exists() {
    filesystem.git_hooks_path().create_dir_all()?;
  }

  let hook_file = filesystem.prepare_commit_msg_path()?;
  let hook_bin = filesystem.git_ai_hook_bin_path()?;

  if hook_file.exists() {
    log::debug!("Removing existing hook file: {}", hook_file);
    hook_file.delete()?;
  }

  hook_file.symlink(hook_bin)?;

  println!(
    "{EMOJI} Hook symlinked successfully to {}",
    hook_file.relative_path()?.to_string().italic()
  );

  Ok(())
}
