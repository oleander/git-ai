use git2::RepositoryOpenFlags as Flags;
use anyhow::{Result, Context, bail};
use std::path::{Path, PathBuf};
use git2::Repository;
use std::env;
use std::fs;

pub fn run() -> Result<()> {
  let current_dir = env::current_dir().with_context(|| "Failed to get current directory".to_string())?;
  let repo = Repository::open_ext(&current_dir, Flags::empty(), Vec::<&Path>::new())
    .with_context(|| "Failed to open repository".to_string())?;

  let hook_dir = PathBuf::from(repo.path()).join("hooks");
  let hook_file = hook_dir.join("prepare-commit-msg");

  if !hook_file.exists() {
    bail!("Hook does not exist: {:?}", hook_file);
  }

  fs::remove_file(&hook_file).with_context(|| format!("Failed to remove file: {:?}", hook_file))?;
  let relative_path = hook_file.strip_prefix(&current_dir).context("Failed to strip prefix")?;
  println!("Hook uninstalled successfully from {:?}", relative_path);

  Ok(())
}
