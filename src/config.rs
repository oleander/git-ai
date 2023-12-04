use anyhow::{Context, Result};

pub fn get_str(key: &str) -> Result<String> {
  Ok(config()?.get_str(key)?.to_string())
}

pub fn get_i32(key: &str) -> Result<i32> {
  config()?.get_i32(key).context("Failed to get config value")
}

pub fn config() -> Result<git2::Config> {
  let path = git2::Config::find_global()?;
  git2::Config::open(path.as_path()).context("Failed to open global git config")
}
