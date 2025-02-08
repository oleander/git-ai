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

use std::str::FromStr;
use std::time::Duration;
use std::path::PathBuf;
use std::process;

use structopt::StructOpt;
use indicatif::ProgressBar;
use anyhow::{bail, Context, Result};
use git2::{Oid, Repository};
use ai::commit;
use ai::hook::*;
use ai::model::Model;
use ai::config;

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
#[structopt(name = "git-ai-hook")]
pub struct Args {
  /// Path to the commit message file
  pub commit_msg_file: PathBuf,

  /// Type of commit message to generate
  #[structopt(short = "t", long = "type")]
  pub commit_type: Option<String>,

  /// SHA1 of the commit to generate message for
  #[structopt(short = "s", long = "sha1")]
  pub sha1: Option<String>
}

impl Args {
  pub async fn handle(&self) -> Result<()> {
    let repo = Repository::open_from_env().context("Failed to open repository")?;
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));

    let app = config::App::new()?;
    let model = app.model.as_deref()
      .map(Model::from)
      .unwrap_or_default();
    let remaining_tokens = commit::token_used(&model)?;

    self
      .handle_commit(&repo, &pb, model, remaining_tokens)
      .await?;
    pb.finish_and_clear();
    Ok(())
  }

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
      .to_commit_diff(tree)?
      .to_patch(remaining_tokens, model)
      .context("Failed to generate patch")?;

    if patch.is_empty() {
      bail!("No changes to commit");
    }

    pb.set_message("Generating commit message...");
    let response = commit::generate(patch, remaining_tokens, model).await?;
    pb.set_message("Writing commit message...");

    self.commit_msg_file.write(&response.response)?;
    Ok(())
  }
}

#[tokio::main]
async fn main() {
  env_logger::init();
  let args = Args::from_args();

  if let Err(e) = args.handle().await {
    eprintln!("Error: {}", e);
    process::exit(1);
  }
}
