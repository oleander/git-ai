use std::sync::Mutex;

use llm_chain::chains::map_reduce::Chain;
use llm_chain::{executor, parameters, prompt};
use git2::{DiffOptions, Repository};
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use llm_chain::step::Step;

// lazy_static! {
//   pub static ref REPO: Mutex<Repository> = Mutex::new(Repository::open_from_env().expect("Failed to open repository"));
// }

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
    let message = self.message().unwrap_or_default();
    let author = self.author().to_string();
    let datetime = self.time().seconds().to_string();
    let id = self.id().to_string();

    let mut commit_info = format!("Commit ID: {}\nAuthor: {}\nDate: {}\nMessage: {}\n\n", id, author, datetime, message);

    // Getting diff
    let tree = self.tree()?;
    let parent = self.parent(0).ok();
    let parent_tree = parent.as_ref().map(|c| c.tree().ok()).flatten();

    let mut opts = DiffOptions::new();
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

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let exec = executor!()?;

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
  let commits = repo.get_last_n_commits(3)?;

  log::info!("Found {} commits", commits.len());

  let docs = commits
    .iter()
    .map(|payload| parameters!("text" => payload.message.clone(), "text" => payload.diff.clone()))
    .collect::<Vec<_>>();

  let res = chain.run(docs, parameters!(), &exec).await.context("Failed to run chain")?;

  println!("{}", res);
  Ok(())
}
