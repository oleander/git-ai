use std::str::FromStr;
use std::time::Duration;
use std::path::PathBuf;

use structopt::StructOpt;
use indicatif::{ProgressBar, ProgressStyle};
use anyhow::{bail, Context, Result};
use git2::{Oid, Repository};
use ai::{commit, config};
use ai::hook::*;
use ai::model::Model;

#[derive(Debug, PartialEq)]
enum Source {
  Message,
  Template,
  Merge,
  Squash,
  Commit
}

impl FromStr for Source {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Self> {
    match s {
      "message" => Ok(Source::Message),
      "template" => Ok(Source::Template),
      "merge" => Ok(Source::Merge),
      "squash" => Ok(Source::Squash),
      "commit" => Ok(Source::Commit),
      other => bail!("{:?} is not a valid source", other)
    }
  }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "commit-msg-hook")]
struct Args {
  /// Name of the file that contains the commit log message
  #[structopt(parse(from_os_str))]
  commit_msg_file: PathBuf,

  /// Source of the commit message
  /// Can be:
  /// - message (if a -m or -F option was given)
  /// - template (if a -t option was given or the configuration option commit.template is set);
  /// - merge (if the commit is a merge or a .git/MERGE_MSG file exists);
  /// - squash (if a .git/SQUASH_MSG file exists);
  /// - commit, followed by a commit object name (if a -c, -C or --amend option was given)
  source: Option<Source>,

  /// Commit object name (optional)
  sha1: Option<String>
}

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();

  let args = Args::from_args();
  let pb = ProgressBar::new_spinner();
  let repo = Repository::open_from_env().context("Failed to open repository")?;
  let model: Model = config::APP.model.clone().into();
  let used_tokens = commit::token_used(&model)?;
  let max_tokens = config::APP.max_tokens.unwrap_or(model.context_size());
  let remaining_tokens = max_tokens.saturating_sub(used_tokens);

  if Some(Source::Message) == args.source && args.sha1.is_some() {
    return Ok(());
  }

  log::debug!("max_tokens: {}", max_tokens);
  log::debug!("used_tokens: {}", used_tokens);

  // Loading bar to indicate that the program is running
  let style = ProgressStyle::default_spinner()
    .tick_strings(&["-", "\\", "|", "/"])
    .template("{spinner:.blue} {msg}")
    .context("Failed to create progress bar style")?;

  pb.set_style(style);
  pb.set_message("Generating commit message...");
  pb.enable_steady_tick(Duration::from_millis(150));

  let tree = match args.sha1.as_deref() {
    // git commit --amend or git commit -c
    Some("HEAD") | None => repo.head().ok().and_then(|head| head.peel_to_tree().ok()),
    // git rebase
    Some(sha1) =>
      repo
        .find_object(Oid::from_str(sha1)?, None)
        .ok()
        .and_then(|obj| obj.peel_to_tree().ok()),
  };

  if remaining_tokens == 0 {
    bail!("No tokens left to generate commit message");
  }

  let patch = repo
    .to_patch(tree, remaining_tokens, model)
    .context("Failed to get patch")?;

  if patch.is_empty() {
    bail!("No changes to commit");
  }

  let pb_clone = pb.clone();
  ctrlc::set_handler(move || {
    pb_clone.finish_and_clear();
    console::Term::stdout()
      .show_cursor()
      .expect("Failed to show cursor");
    std::process::exit(1);
  })?;

  pb.set_message("Generating commit message...");

  let response = commit::generate(patch.to_string(), remaining_tokens, model).await?;
  std::fs::write(&args.commit_msg_file, response.response.trim())?;
  pb.finish_and_clear();

  Ok(())
}
