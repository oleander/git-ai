#![allow(dead_code)]

// .git/hooks/prepare-commit-msg
//
// git commit --amend --no-edit
// Args { commit_msg_file: PathBuf::from(".git/COMMIT_EDITMSG"), source: Some(Source::Commit), sha1: Some("HEAD") }
// Outcome: The previous commit message is reused without opening the editor. No new commit message is generated.

// git commit --amend -m 'Initial commit'
// Args { commit_msg_file: PathBuf::from(".git/COMMIT_EDITMSG"), source: Some(Source::Message), sha1: Some("HEAD") }
// Outcome: The commit message 'Initial commit' is set directly from the command line. No new commit message is generated.

// git commit -m 'Initial commit'
// Args { commit_msg_file: PathBuf::from(".git/COMMIT_EDITMSG"), source: Some(Source::Message), sha1: None }
// Outcome: The commit message 'Initial commit' is set directly from the command line. No new commit message is generated.

// git commit --no-edit
// Args { commit_msg_file: PathBuf::from(".git/COMMIT_EDITMSG"), source: None, sha1: None }
// Outcome: No commit message is provided. It will use the existing commit message if allowed. No new commit message is generated.

// git commit -c HEAD^2
// Args { commit_msg_file: PathBuf::from(".git/COMMIT_EDITMSG"), source: Some(Source::Commit), sha1: Some("HEAD^2") }
// Outcome: Opens the commit message editor with the message from the specified commit (HEAD^2). No new commit message is generated automatically; it depends on user input.

// git commit -c HEAD^2 --no-edit
// Args { commit_msg_file: PathBuf::from(".git/COMMIT_EDITMSG"), source: Some(Source::Commit), sha1: Some("HEAD^2") }
// Outcome: Uses the commit message from the specified commit (HEAD^2) without opening the editor. No new commit message is generated.

// git commit --squash HEAD^3
// Args { commit_msg_file: PathBuf::from(".git/SQUASH_MSG"), source: Some(Source::Squash), sha1: Some("HEAD^3") }
// Outcome: Squashes the last 3 commits into a single commit. A new commit message is generated as there is no provided commit message.

// git commit --no-edit --no-verify -m 'Merge branch 'feature-branch''
// Args { commit_msg_file: PathBuf::from(".git/MERGE_MSG"), source: Some(Source::Merge), sha1: None }
// Outcome: Merges the feature-branch into the main branch. Git automatically generates a commit message in the format 'Merge branch 'feature-branch''.

// git commit -t template.txt
// Args { commit_msg_file: PathBuf::from(".git/COMMIT_EDITMSG"), source: Some(Source::Template), sha1: None }
// Outcome: Commits changes using the commit message from the template.txt file. No new commit message is generated.

// git commit --amend
// Args { commit_msg_file: PathBuf::from(".git/COMMIT_EDITMSG"), source: Some(Source::Commit), sha1: Some("HEAD") }
// Outcome: Opens the default text editor to allow modification of the most recent commit message. No new commit message is generated automatically; it depends on user input.
use std::process::exit;
use std::str::FromStr;
use std::time::Duration;
use std::path::PathBuf;

#[cfg(debug_assertions)]
use colored::Colorize;
use structopt::StructOpt;
use indicatif::{ProgressBar, ProgressStyle};
use anyhow::{bail, Context, Result};
use git2::{Oid, Repository};
use ai::{commit, config, debug_output};
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
  #[structopt(parse(from_os_str))]
  commit_msg_file: PathBuf,
  source:          Option<Source>,
  sha1:            Option<String>
}

impl Args {
  async fn handle_commit(&self, repo: &Repository, pb: &ProgressBar, model: Model, remaining_tokens: usize) -> Result<()> {
    let tree = match self.sha1.as_deref() {
      Some("HEAD") | None => repo.head().ok().and_then(|head| head.peel_to_tree().ok()),
      Some(sha1) => {
        // Try to resolve the reference first
        if let Ok(obj) = repo.revparse_single(sha1) {
          obj.peel_to_tree().ok()
        } else {
          // If not a reference, try as direct OID
          repo
            .find_object(Oid::from_str(sha1)?, None)
            .ok()
            .and_then(|obj| obj.peel_to_tree().ok())
        }
      }
    };

    let diff = repo.to_diff(tree.clone())?;
    if diff.is_empty()? {
      if self.sha1.as_deref() == Some("HEAD") {
        // For amend operations, we want to keep the existing message
        return Ok(());
      }
      bail!("No changes to commit");
    }

    let patch = repo
      .to_patch(tree, remaining_tokens, model.clone())
      .context("Failed to get patch")?;

    let response = commit::generate(patch.to_string(), remaining_tokens, model, None).await?;
    std::fs::write(&self.commit_msg_file, response.response.trim())?;

    pb.finish_and_clear();

    Ok(())
  }

  async fn execute(&self) -> Result<()> {
    use Source::*;

    match self.source {
      Some(Message | Template | Merge | Squash) => Ok(()),
      Some(Commit) | None => {
        let repo = Repository::open_from_env().context("Failed to open repository")?;
        let model = config::APP
          .model
          .clone()
          .unwrap_or("gpt-4o-mini".to_string())
          .into();
        let used_tokens = commit::token_used(&model)?;
        let max_tokens = config::APP.max_tokens.unwrap_or(model.context_size());
        let remaining_tokens = max_tokens.saturating_sub(used_tokens).max(512); // Ensure minimum 512 tokens

        let tree = match self.sha1.as_deref() {
          Some("HEAD") | None => repo.head().ok().and_then(|head| head.peel_to_tree().ok()),
          Some(sha1) => {
            // Try to resolve the reference first
            if let Ok(obj) = repo.revparse_single(sha1) {
              obj.peel_to_tree().ok()
            } else {
              // If not a reference, try as direct OID
              repo
                .find_object(Oid::from_str(sha1)?, None)
                .ok()
                .and_then(|obj| obj.peel_to_tree().ok())
            }
          }
        };

        let diff = repo.to_diff(tree.clone())?;
        if diff.is_empty()? {
          if self.source == Some(Commit) {
            // For amend operations, we want to keep the existing message
            return Ok(());
          }
          bail!("No changes to commit");
        }

        let pb = ProgressBar::new_spinner();
        let style = ProgressStyle::default_spinner()
          .tick_strings(&["-", "\\", "|", "/"])
          .template("{spinner:.blue} {msg}")
          .context("Failed to create progress bar style")?;

        pb.set_style(style);
        pb.set_message("Generating commit message...");
        pb.enable_steady_tick(Duration::from_millis(150));

        // Check if a commit message already exists and is not empty
        if !std::fs::read_to_string(&self.commit_msg_file)?
          .trim()
          .is_empty()
        {
          log::debug!("A commit message has already been provided");
          pb.finish_and_clear();
          return Ok(());
        }

        let patch = repo
          .to_patch(tree, remaining_tokens, model.clone())
          .context("Failed to get patch")?;

        let response = commit::generate(patch.to_string(), remaining_tokens, model, None).await?;
        std::fs::write(&self.commit_msg_file, response.response.trim())?;

        pb.finish_and_clear();

        Ok(())
      }
    }
  }
}

#[tokio::main]
async fn main() -> Result<()> {
  // Always enable debug logging in debug builds
  #[cfg(debug_assertions)]
  {
    if std::env::var("RUST_LOG").is_err() {
      std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();
  }

  // In release builds, only initialize logging if RUST_LOG is set
  #[cfg(not(debug_assertions))]
  if std::env::var("RUST_LOG").is_ok() {
    env_logger::init();
  }

  let time = std::time::Instant::now();
  let args = Args::from_args();

  // Initialize debug session if in debug mode
  if log::log_enabled!(log::Level::Debug) {
    let args_str = format!(
      "commit_msg_file='{}', source={:?}, sha1={:?}",
      args.commit_msg_file.display(),
      args.source,
      args.sha1
    );
    debug_output::init_debug_session(&args_str);
  }

  let result = args.execute().await;
  let total_time = time.elapsed();

  if let Err(err) = result {
    eprintln!("{err} ({total_time:?})");
    exit(1);
  } else {
    // In debug builds, show comprehensive timing report
    #[cfg(debug_assertions)]
    {
      if log::log_enabled!(log::Level::Debug) {
        debug_output::print_final_output();
      } else {
        eprintln!("\n{}", "Performance Summary:".blue().bold());
        eprintln!("├─ Total execution time: {total_time:?}");
        eprintln!("└─ See detailed section timings above");
      }
    }
    // In release builds with debug logging enabled
    #[cfg(not(debug_assertions))]
    if log::log_enabled!(log::Level::Debug) {
      debug_output::print_final_output();
    }
  }

  Ok(())
}
