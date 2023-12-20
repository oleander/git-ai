use std::sync::Mutex;

use llm_chain::{options, parameters, prompt};
use llm_chain::chains::map_reduce::Chain;
use git2::{DiffOptions, Repository};
use llm_chain::traits::Executor;
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use llm_chain::step::Step;

lazy_static! {
  pub static ref REPO: Mutex<Repository> = Mutex::new(Repository::open_from_env().expect("Failed to open repository"));
}

#[derive(Debug, Clone)]
struct Payload {
  pub message: String,
  pub diff:    String
}

trait CommitExt {
  fn show(&self, repo: &Repository) -> Result<String, git2::Error>;
}

impl CommitExt for git2::Commit<'_> {
  fn show(&self, repo: &Repository) -> Result<String, git2::Error> {
    let mut commit_info = "".to_string();
    let mut opts = DiffOptions::new();
    let tree = self.tree()?;
    let parent_tree = self.parent(0).ok().as_ref().map(|c| c.tree().ok()).flatten();
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))?;

    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
      commit_info.push_str(std::str::from_utf8(line.content()).unwrap());
      true
    })?;

    Ok(commit_info)
  }
}

trait RepositoryExt {
  fn get_last_n_commits(&self, n: usize) -> Result<Vec<Payload>, git2::Error>;
}

impl RepositoryExt for Repository {
  fn get_last_n_commits(&self, n: usize) -> Result<Vec<Payload>, git2::Error> {
    let mut revwalk = self.revwalk()?;
    revwalk.push_head()?;
    Ok(
      revwalk
        .take(n)
        .map(move |id| {
          let commit = self.find_commit(id.unwrap()).expect("Failed to find commit");
          Payload {
            message: commit.message().unwrap().to_string(), diff: commit.show(&self).unwrap()
          }
        })
        .collect()
    )
  }
}

const DEFAULT_MAX_COMMITS: u8 = 10;
const DEFAULT_MAX_TOKENS: u16 = 5000;

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
  #[arg(long)]
  max_commits: Option<u8>,

  #[arg(long)]
  max_tokens: Option<u16>
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cli = Cli::parse();
  let max_tokens = cli.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);
  let options = options!(MaxTokens: max_tokens);
  let exec = llm_chain_openai::chatgpt::Executor::new_with_options(options);

  env_logger::init();

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
  let commits = repo.get_last_n_commits(cli.max_commits.unwrap_or(DEFAULT_MAX_COMMITS) as usize)?;

  log::info!("Found {} commits", commits.len());

  let docs = commits
    .iter()
    .map(|payload| parameters!("text" => payload.message.clone(), "text" => payload.diff.clone()))
    .collect::<Vec<_>>();

  let res = chain.run(docs, parameters!(), &exec.unwrap()).await.context("Failed to run chain")?;

  println!("{}", res);
  Ok(())
}
