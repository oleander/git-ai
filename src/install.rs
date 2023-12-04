use git2::RepositoryOpenFlags as Flags;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use git2::Repository;
use std::env;
use std::fs;
use log::debug;

// Git hook: prepare-commit-msg
// Crates an executable git hook (prepare-commit-msg) in the .git/hooks directory
pub fn run() -> Result<()> {
  let script = include_bytes!("../target/release/git-ai-hook");

  let current_dir = env::current_dir().with_context(|| "Failed to get current directory".to_string())?;
  debug!("Current directory: {:?}", current_dir);

  let repo = Repository::open_ext(&current_dir, Flags::empty(), Vec::<&Path>::new())
    .with_context(|| "Failed to open repository".to_string())?;
  debug!("Repository path: {:?}", repo.path());

  let hook_dir = PathBuf::from(repo.path()).join("hooks");
  let hook_file = hook_dir.join("prepare-commit-msg");
  debug!("Hook file: {:?}", hook_file);

  debug!("Creating directory: {:?}", hook_dir);
  fs::create_dir_all(&hook_dir).with_context(|| format!("Failed to create directory: {:?}", hook_dir))?;

  if hook_file.exists() {
    debug!("Removing file: {:?}", hook_file);
    fs::remove_file(&hook_file).with_context(|| format!("Failed to remove file: {:?}", hook_file))?;
  }

  debug!("Writing file: {:?}", hook_file);
  fs::write(&hook_file, script).with_context(|| format!("Failed to write file: {:?}", hook_file))?;

  let metadata = fs::metadata(&hook_file).context("Failed to get metadata")?;
  let mut permissions = metadata.permissions();
  debug!("Current permissions: {:?}", permissions);

  permissions.set_mode(0o755);
  debug!("New permissions: {:?}", permissions);
  fs::set_permissions(&hook_file, permissions).context("Failed to set permissions")?;

  let relative_path = hook_file.strip_prefix(&current_dir).context("Failed to strip prefix")?;

  println!("Hook symlinked successfully to {}", relative_path.display());

  Ok(())
}
