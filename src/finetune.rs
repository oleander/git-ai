use std::fs;
use std::io::Write;
use std::sync::Arc;
use std::collections::HashSet;

use anyhow::{Context, Result};
use colored::*;
use git2::{DiffOptions, Repository};
use indicatif::{ProgressBar, ProgressStyle};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;
use tokio::sync::{mpsc, Mutex};
use tokio::task;
use num_cpus;

use crate::model::Model;
use crate::openai;

/// Represents command-line arguments for fine-tuning
#[derive(Debug, Clone, Deserialize, Serialize, StructOpt)]
pub struct FinetuneArgs {
  #[structopt(long, default_value = "resources/prompt.md")]
  pub prompt_file: String,

  #[structopt(long, default_value = "finetune_train.jsonl")]
  pub train_file: String,

  #[structopt(long, default_value = "finetune_verify.jsonl")]
  pub verify_file: String,

  #[structopt(long, default_value = "50")]
  pub num_commits: u32,

  #[structopt(long)]
  pub parallel_requests: Option<usize>,

  #[structopt(long, default_value = "0.8")]
  pub quality_threshold: f32,

  #[structopt(long)]
  pub verbose: bool,

  #[structopt(long, default_value = "5000")]
  pub max_diff_size: usize
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
  role:    String,
  content: String
}

#[derive(Debug, Serialize, Deserialize)]
struct TrainingExample {
  messages: Vec<Message>
}

/// Track the types of changes in a commit
#[derive(Debug)]
struct CommitChangeTypes {
  #[allow(dead_code)]
  has_additions:         bool,
  #[allow(dead_code)]
  has_deletions:         bool,
  #[allow(dead_code)]
  has_modifications:     bool,
  #[allow(dead_code)]
  has_renames:           bool,
  #[allow(dead_code)]
  has_file_mode_changes: bool
}

/// Simple container for commit info
#[derive(Debug)]
struct CommitInfo {
  message:      String,
  diff:         String,
  #[allow(dead_code)]
  change_types: CommitChangeTypes
}

pub async fn run(args: FinetuneArgs) -> Result<()> {
  println!("🔄 Starting fine-tuning data export...");

  // Reset (truncate) the output files
  fs::write(&args.train_file, "")?;
  fs::write(&args.verify_file, "")?;

  // Track seen messages to prevent duplicates
  let seen_messages = Arc::new(Mutex::new(HashSet::new()));

  // 1. Load system prompt
  let prompt_content =
    fs::read_to_string(&args.prompt_file).with_context(|| format!("Failed to read prompt file: {}", args.prompt_file))?;

  // 2. Open local repository and setup commit processing
  println!("📚 Collecting commit history...");
  let repo = Repository::open(".")?;
  let mut revwalk = repo.revwalk()?;
  revwalk.push_head()?;

  let mut total_checked = 0;
  let mut valid_commits = 0;
  let mut commit_data = Vec::new();

  let collect_pb = ProgressBar::new_spinner();
  collect_pb.set_style(
    ProgressStyle::default_spinner()
      .template("{spinner:.green} Processing commits: {pos} found ({msg})")
      .unwrap()
  );

  // Process commits as we find them
  for oid in revwalk {
    total_checked += 1;
    if let Ok(id) = oid {
      if let Ok(commit) = repo.find_commit(id) {
        let message = commit.message().unwrap_or("");
        if (20..500).contains(&message.len()) && commit.parent_count() == 1 {
          let parent = commit.parent(0)?;
          let parent_tree = parent.tree()?;
          let commit_tree = commit.tree()?;
          let mut diff_opts = DiffOptions::new();
          let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), Some(&mut diff_opts))?;

          let mut diff_text = String::new();
          let mut total_diff_size = 0;
          let mut should_skip = false;

          diff.print(git2::DiffFormat::Patch, |_, _, line| {
            if let Ok(content) = std::str::from_utf8(line.content()) {
              total_diff_size += content.len();
              if total_diff_size <= args.max_diff_size {
                diff_text.push(line.origin());
                diff_text.push_str(content);
              } else {
                should_skip = true;
              }
            }
            true
          })?;

          if !should_skip {
            commit_data.push((message.to_string(), diff_text));
            valid_commits += 1;
            collect_pb.set_position(valid_commits as u64);
            collect_pb.set_message(format!("latest: {:.40}...", message));
          }
        }
      }
    }
    if valid_commits >= args.num_commits as usize * 3 {
      break;
    }
  }

  if args.verbose {
    println!("    Checked {} commits, found {} valid ones", total_checked, valid_commits);
  }
  collect_pb.finish_with_message(format!("Found {} commits to process", valid_commits));

  // Shuffle the collected commits for randomization
  let mut rng = rand::rngs::ThreadRng::default();
  commit_data.shuffle(&mut rng);
  let commit_data = Arc::new(commit_data);

  // Setup processing channel
  let num_workers = args.parallel_requests.unwrap_or_else(num_cpus::get);
  let (tx, mut rx) = mpsc::channel(num_workers * 2);
  let approved_commits = Arc::new(Mutex::new(0usize));
  let threshold = args.quality_threshold;

  // Create progress bar for approved commits
  let process_pb = ProgressBar::new(args.num_commits as u64);
  process_pb.set_style(
    ProgressStyle::default_bar()
      .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} approved ({eta})")
      .unwrap()
      .progress_chars("#>-")
  );

  // Spawn workers for quality checking
  let mut workers = Vec::new();
  for worker_id in 0..num_workers {
    let tx = tx.clone();
    let approved = Arc::clone(&approved_commits);
    let seen = Arc::clone(&seen_messages);
    let pb = process_pb.clone();
    let verbose = args.verbose;
    let target_commits = args.num_commits;
    let commit_data = Arc::clone(&commit_data);
    let start_idx = worker_id * commit_data.len() / num_workers;
    let end_idx = ((worker_id + 1) * commit_data.len() / num_workers).min(commit_data.len());

    let worker = task::spawn(async move {
      for (message, diff) in commit_data[start_idx..end_idx].iter() {
        let current_approved = {
          let count = approved.lock().await;
          *count
        };
        if current_approved >= target_commits as usize {
          break;
        }
        let is_duplicate = {
          let mut seen = seen.lock().await;
          if seen.contains(message) {
            true
          } else {
            seen.insert(message.clone());
            false
          }
        };
        if !is_duplicate {
          if let Ok(score) = rate_commit_quality(&CommitInfo {
            message:      message.clone(),
            diff:         diff.clone(),
            change_types: CommitChangeTypes {
              has_additions:         false,
              has_deletions:         false,
              has_modifications:     false,
              has_renames:           false,
              has_file_mode_changes: false
            }
          })
          .await
          {
            if score >= threshold {
              if let Ok(cleaned_message) = cleanup_commit_message(message).await {
                let mut count = approved.lock().await;
                *count += 1;
                pb.set_position(*count as u64);
                if verbose {
                  println!("✓ {} (score: {:.2})", cleaned_message.bright_green(), score);
                }
                if tx.send((message.clone(), diff.clone())).await.is_err() {
                  break;
                }
              }
            }
          }
        }
      }
    });
    workers.push(worker);
  }
  drop(tx);

  // Process approved commits
  let mut approved_count = 0;
  let train_size = args.num_commits / 2;
  let mut train_file = fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(&args.train_file)?;
  let mut verify_file = fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(&args.verify_file)?;

  while let Some((message, diff)) = rx.recv().await {
    if approved_count >= args.num_commits as usize {
      break;
    }
    let cleaned_message = cleanup_commit_message(&message).await?;
    if cleaned_message.trim().is_empty() {
      continue;
    }
    let is_duplicate = {
      let mut seen = seen_messages.lock().await;
      if seen.contains(&cleaned_message) {
        true
      } else {
        seen.insert(cleaned_message.clone());
        false
      }
    };
    if is_duplicate {
      continue;
    }
    // Run scoring on the cleaned output
    let cleaned_score = rate_cleaned_commit_message(&cleaned_message).await?;
    if args.verbose {
      println!("Cleaned: {} (score: {:.2})", cleaned_message, cleaned_score);
    }
    let example = TrainingExample {
      messages: vec![
        Message {
          role:    "system".to_string(),
          content: prompt_content.clone()
        },
        Message { role: "user".to_string(), content: diff },
        Message {
          role:    "assistant".to_string(),
          content: cleaned_message
        },
      ]
    };
    let json = serde_json::to_string(&example)?;
    if approved_count < train_size as usize {
      writeln!(train_file, "{}", json)?;
    } else {
      writeln!(verify_file, "{}", json)?;
    }
    approved_count += 1;
  }

  for worker in workers {
    worker.await?;
  }
  process_pb.finish();

  println!("\n✨ Successfully exported {} training examples:", approved_count);
  println!("   - {} training examples in {}", train_size, args.train_file);
  println!("   - {} verification examples in {}", args.num_commits - train_size, args.verify_file);

  Ok(())
}

/// Cleanup commit message using GPT4oMini
async fn cleanup_commit_message(original_msg: &str) -> Result<String> {
  if original_msg.trim().is_empty() {
    return Ok(String::new());
  }
  let first_line = original_msg
    .lines()
    .next()
    .unwrap_or("")
    .trim()
    .trim_start_matches("```")
    .trim_end_matches("```")
    .trim_start_matches("plaintext")
    .trim_start_matches("git")
    .trim();
  let system_prompt = "\
You are an expert at cleaning up git commit messages. \
Your task is to:\n\
1. Remove any ticket references or extraneous tags\n\
2. Keep it short, focusing on meaningful description\n\
3. Do not end the message with a period\n\
4. Always start with a capitalized verb (Add, Fix, Update, etc)\n\
5. Drop the type prefix if it is present\n\
6. Return ONLY the cleaned message without any formatting or backticks";
  let req = openai::Request {
    system:     system_prompt.to_string(),
    prompt:     first_line.to_string(),
    max_tokens: 100,
    model:      Model::GPT4oMini
  };
  let response = openai::call(req).await?;
  let cleaned = response
    .trim()
    .trim_start_matches("```")
    .trim_end_matches("```")
    .trim_start_matches("plaintext")
    .trim_start_matches("git")
    .trim()
    .to_string();
  if cleaned.is_empty()
    || cleaned.to_lowercase().contains("please")
    || cleaned.to_lowercase().contains("provide")
    || cleaned.to_lowercase().contains("didn't")
    || cleaned.to_lowercase().contains("error")
    || cleaned.to_lowercase().contains("missing")
    || cleaned.to_lowercase().contains("sorry")
    || cleaned.to_lowercase().contains("unable")
    || cleaned.to_lowercase().contains("could not")
    || cleaned.to_lowercase().contains("cannot")
    || cleaned.to_lowercase().contains("failed")
    || cleaned.len() > 100
  {
    return Ok(String::new());
  }
  let message = if cleaned.contains(": ") {
    let parts: Vec<&str> = cleaned.splitn(2, ": ").collect();
    parts.get(1).unwrap_or(&cleaned.as_str()).trim().to_string()
  } else {
    cleaned
  };
  let mut chars = message.chars();
  Ok(if let Some(first_char) = chars.next() {
    if first_char.is_lowercase() {
      first_char.to_uppercase().collect::<String>() + chars.as_str()
    } else {
      message
    }
  } else {
    message
  })
}

/// Rate commit quality using GPT4oMini
async fn rate_commit_quality(commit_info: &CommitInfo) -> Result<f32> {
  let system_prompt = "\
You are an expert at evaluating git commit quality. Your task is to rate this commit from 0.0 to 1.0 based on:

1. Commit Message Quality (50% of score):
   - Is the first line concise (under 72 chars)?
   - If present, is the body descriptive and separated by blank line?
   - Is the message present tense?
   - Is the message written in the active voice?
   - Is the message clear and concise?

2. Diff Alignment (50% of score):
   - Does the message accurately describe the changes in the diff?
   - Are all significant changes reflected in the message?
   - Is the scope of changes consistent with the message?

Scoring Guide:
- 0.0-0.3: Poor quality (wrong format, unclear or misleading, conventional commit format)
- 0.4-0.6: Mediocre quality (basic description)
- 0.7-0.8: Good quality (follows format, clear message, mostly aligned with changes)
- 0.9-1.0: Excellent (perfect format and description of changes)

Return ONLY a number between 0.0 and 1.0";
  let prompt = format!(
    "Evaluate this commit:\n\nCommit Message:\n{}\n\nCode Changes:\n{}\n\nScore (0.0-1.0):",
    commit_info.message, commit_info.diff
  );
  let req = openai::Request {
    system: system_prompt.to_string(),
    prompt,
    max_tokens: 10,
    model: Model::GPT4oMini
  };
  let response = openai::call(req).await?;
  let score = response.trim().parse::<f32>().unwrap_or(0.0);
  Ok(score.clamp(0.0, 1.0))
}

/// Rate cleaned commit message quality using GPT4oMini
async fn rate_cleaned_commit_message(cleaned_message: &str) -> Result<f32> {
  let system_prompt = "\
You are an expert at evaluating cleaned git commit messages. Rate the quality of this commit message on a scale from 0.0 to 1.0, based solely on clarity, conciseness, and adherence to conventional commit style guidelines. Return ONLY a number between 0.0 and 1.0.";
  let prompt = format!("Cleaned Commit Message:\n{}\nScore (0.0-1.0):", cleaned_message);
  let req = openai::Request {
    system: system_prompt.to_string(),
    prompt,
    max_tokens: 10,
    model: Model::GPT4oMini
  };
  let response = openai::call(req).await?;
  let score = response.trim().parse::<f32>().unwrap_or(0.0);
  Ok(score.clamp(0.0, 1.0))
}
