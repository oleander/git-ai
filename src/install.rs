use git2::RepositoryOpenFlags as Flags;
use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::os::unix::fs::PermissionsExt; // This trait provides the set_mode method
use git2::Repository;
use std::env;
use std::fs;

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

  //   unix_fs::symlink(&binary_path, &hook_file).with_context(|| format!("Failed to create symlink: {:?}", hook_file))?;
  // write the script to the file
  fs::write(&hook_file, script).with_context(|| format!("Failed to write file: {:?}", hook_file))?;

  let metadata = fs::metadata(&hook_file)?;
  let mut permissions = metadata.permissions();
  permissions.set_mode(0o755); // Read/write for owner and read for others.
  fs::set_permissions(&hook_file, permissions)?;

  let relative_path = hook_file.strip_prefix(&current_dir).context("Failed to strip prefix")?;
  println!("Hook symlinked successfully to {:?}", relative_path);

  Ok(())
}
