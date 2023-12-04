use git2::RepositoryOpenFlags as Flags;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use git2::Repository;
use log::info;
use std::env;
use std::fs;

// Git hook: prepare-commit-msg
pub fn run() -> Result<()> {
  let current_dir = env::current_dir().with_context(|| "Failed to get current directory".to_string())?;
  let repo = Repository::open_ext(&current_dir, Flags::empty(), Vec::<&Path>::new())
    .with_context(|| "Failed to open repository".to_string())?;
  let script = include_bytes!("../target/release/git-ai-hook");
  let hook_dir = PathBuf::from(repo.path()).join("hooks");
  let hook_file = hook_dir.join("prepare-commit-msg");

  fs::create_dir_all(&hook_dir).with_context(|| format!("Failed to create directory: {:?}", hook_dir))?;

  if hook_file.exists() {
    fs::remove_file(&hook_file).with_context(|| format!("Failed to remove file: {:?}", hook_file))?;
  }

  fs::write(&hook_file, script).with_context(|| format!("Failed to write file: {:?}", hook_file))?;
  let metadata = fs::metadata(&hook_file).context("Failed to get metadata")?;
  let mut permissions = metadata.permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&hook_file, permissions).context("Failed to set permissions")?;

  let relative_path = hook_file.strip_prefix(&current_dir).context("Failed to strip prefix")?;
  info!("Hook symlinked successfully to {:?}", relative_path);

  Ok(())
}
