use std::sync::Mutex;
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};
use llm_chain::{options, parameters, prompt};
use llm_chain::chains::map_reduce::Chain;
use git2::{DiffOptions, Repository};
use llm_chain::traits::Executor;
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use llm_chain::step::Step;
use clap::{Parser, ArgMatches};
use ai::config::APP;

lazy_static! {
  pub static ref REPO: Mutex<Repository> = Mutex::new(Repository::open_from_env().expect("Failed to open repository"));
}

#[derive(Debug, Clone)]
struct Payload {
  pub message: String,
  pub diff:    String
}

trait CommitExt {
  fn show(&self, repo: &Repository, max_tokens: usize) -> Result<String, git2::Error>;
}

impl CommitExt for git2::Commit<'_> {
  fn show(&self, repo: &Repository, max_tokens: usize) -> Result<String, git2::Error> {
    let mut commit_info = "".to_string();
    let mut opts = DiffOptions::new();
    let tree = self.tree()?;
    let parent_tree = self.parent(0).ok().as_ref().map(|c| c.tree().ok()).flatten();
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))?;

    _ = diff
      .print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        commit_info.push_str(std::str::from_utf8(line.content()).unwrap());
        commit_info.len() < max_tokens
      })
      .ok();

    Ok(commit_info)
  }
}

trait RepositoryExt {
  fn get_last_n_commits(&self, max_commits: usize, max_tokens: usize) -> Result<Vec<Payload>, git2::Error>;
}

impl RepositoryExt for Repository {
  fn get_last_n_commits(&self, max_commits: usize, max_tokens: usize) -> Result<Vec<Payload>, git2::Error> {
    let mut revwalk = self.revwalk()?;
    revwalk.push_head()?;
    Ok(
      revwalk
        .take(max_commits)
        .map(move |id| {
          let commit = self.find_commit(id.unwrap()).expect("Failed to find commit");
          Payload {
            message: commit.message().unwrap().to_string(), diff: commit.show(&self, max_tokens).unwrap()
          }
        })
        .collect()
    )
  }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
  #[arg(long, default_value = "10")]
  max_commits: Option<u8>,

  #[arg(long, default_value = "4500")]
  max_tokens: Option<u16>
}

pub async fn run(args: &ArgMatches) -> Result<()> {
  let max_commits: usize =  *args.get_one("max-commits").unwrap_or(&10);
  let max_tokens: usize =  *args.get_one("max-tokens").unwrap_or(&4500);
  let api_key = APP.openai_api_key.clone().context("Failed to get OpenAI API key, please run `git-ai config set openapi-api-key <api-key>`")?;

  let options = options!(MaxTokens: max_tokens, MaxContextSize: max_tokens, ApiKey: api_key);
  let exec = llm_chain_openai::chatgpt::Executor::new_with_options(options);

  let style = ProgressStyle::default_spinner()
    .tick_strings(&["-", "\\", "|", "/"])
    .template("{spinner:.blue} {msg}")
    .context("Failed to create progress bar style")?;

  let pb = ProgressBar::new_spinner();
  pb.set_style(style);
  pb.set_message("Building chain...");
  pb.enable_steady_tick(Duration::from_millis(150));

  let map_prompt = Step::for_prompt_template(prompt!(
    "You are an AI trained to analyze code diffs and generate commit messages that match the style and tonality of previous commits.",
    "Given the context of the previous commit message: '{{text}}' analyze this code diff: '{{text}}', and suggest a new commit message that maintains a similar style and tone."
  ));

  let reduce_prompt = Step::for_prompt_template(prompt!(
    "You are an AI summarizing multiple code changes in the context of past commits for a comprehensive commit message.",
    "Combine these change analyses with the context of the last commit message: '{{text}}' into a cohesive new commit message."
  ));

  let repo = REPO.lock().unwrap();
  let chain = Chain::new(map_prompt, reduce_prompt);
  let commits = repo.get_last_n_commits(max_commits, max_tokens).unwrap();

  log::info!("Found {} commits", commits.len());

  let docs = commits
    .iter()
    .map(|payload| parameters!("text" => payload.message.clone(), "text" => payload.diff.clone()))
    .collect::<Vec<_>>();

  let key = "ai.history";
  let data = chain.run(docs, parameters!(), &exec.unwrap()).await.context("Failed to run chain")?;
  let str = data
    .to_immediate()
    .await
    .context("Failed to convert data to immediate")?
    .primary_textual_output()
    .unwrap();

  let value = hex::encode(str);
  let mut config = repo.config()?;

  config.set_str(key, value.as_str())?;

  log::info!("Wrote {} bytes to {}", value.len(), key);

  Ok(())
}
