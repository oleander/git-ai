use anyhow::{bail, Result};
use ai::filesystem::Filesystem;

#[allow(dead_code)]
pub fn run() -> Result<()> {
  let fs = Filesystem::new()?;
  let hook_bin = fs.git_ai_hook_bin_path()?;
  let hook_file = fs.prepare_commit_msg_path()?;

  if hook_file.exists() {
    bail!("Hook already exists at {}, please run 'git ai hook reinstall'", hook_file);
  }

  hook_file.symlink(&hook_bin)?;
  println!("🔗 Hook symlinked successfully to {}", hook_file);

  Ok(())
}
