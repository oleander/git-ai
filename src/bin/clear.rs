use git2::Repository;
use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();

  let repo = Repository::open_from_env().context("Failed to open repository")?;
  let mut config = repo.config().context("Failed to load config")?;
  config.remove("ai.thread-id").context("Failed to delete thread-id")?;
  config.snapshot().context("Failed to save config")?;
  let mut global_config = config.open_global().context("Failed to open global config")?;
  global_config
    .remove("ai.assistant-id")
    .context("Failed to delete assistant-id")?;
  global_config.snapshot().context("Failed to save global config")?;
  Ok(())
}
