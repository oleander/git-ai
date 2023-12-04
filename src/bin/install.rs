use std::fs;
use std::path::Path;
use anyhow::{Result, Context, bail};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
  let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
  let current_dir = env::current_dir().with_context(|| format!("Failed to get current directory"))?;
  let repo = git2::Repository::open_ext(current_dir.clone(), git2::RepositoryOpenFlags::empty(), Vec::<&Path>::new())
    .with_context(|| format!("Failed to open repository"))?;
  let binary_path = current_dir.join(format!("target/{}/hook", profile));

  if !binary_path.exists() {
    bail!("Binary does not exist: {:?}", binary_path);
  }

  let hook_dir = repo.path().join("hooks");
  let hook_file = hook_dir.join("prepare-commit-msg");

  let binary_contents = fs::read(&binary_path).with_context(|| format!("Failed to read binary file: {:?}", binary_path))?;

  fs::create_dir_all(&hook_dir).with_context(|| format!("Failed to create directory: {:?}", hook_dir))?;

  fs::write(&hook_file, binary_contents).with_context(|| format!("Failed to write to file: {:?}", hook_file))?;

  let relative_path = hook_file.strip_prefix(&current_dir).context("Failed to strip prefix")?;
  println!("Hook installed successfully in {:?}", relative_path);

  Ok(())
}
