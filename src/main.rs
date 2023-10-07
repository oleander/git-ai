use git2::Repository;
use std::env;
use std::path::Path;
use std::process::{exit, Command};

fn main() {
  if !ensure_aicommit_hooks_installed() {
    eprintln!("Error: aicommit hooks are not installed.");
    exit(1);
  }

  let files_to_add = get_git_status();

  if files_to_add.is_empty() {
    println!("No changes detected.");
    exit(0);
  }

  if should_add_all() && !run_git_add() {
    eprintln!("Error adding changes to git.");
    exit(1);
  }

  match run_git_commit() {
    Ok(_) => {},
    Err(err) => {
      eprintln!("Error committing changes: {}", err);
      exit(1);
    },
  }

  let commit_message = match get_latest_commit_message() {
    Ok(message) => message,
    Err(err) => {
      eprintln!("Failed to fetch the latest commit message: {}", err);
      exit(1);
    },
  };

  println!("▶ {}", commit_message);

  for line in files_to_add {
    println!("   🔸{}", line);
  }
}

fn get_git_status() -> Vec<String> {
  let output = Command::new("git")
    .arg("status")
    .arg("--porcelain")
    .output()
    .expect("Failed to execute git status");

  String::from_utf8_lossy(&output.stdout).lines().map(String::from).collect()
}

fn run_git_add() -> bool {
  let output = Command::new("git").arg("add").arg(".").output().expect("Failed to execute git add");

  output.status.success()
}

fn run_git_commit() -> Result<(), String> {
  let output = Command::new("git").arg("commit").arg("--no-edit").output();

  match output {
    Ok(output) => {
      if output.status.success() {
        Ok(())
      } else {
        let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if err_msg.is_empty() {
          Err(String::from("Git commit failed for an unknown reason."))
        } else {
          Err(err_msg)
        }
      }
    },
    Err(err) => Err(err.to_string()),
  }
}

fn get_latest_commit_message() -> Result<String, git2::Error> {
  let repo = Repository::open(".")?;
  let head = repo.head()?;
  let commit = head.peel_to_commit()?;
  Ok(commit.message().unwrap_or("").to_string())
}

fn ensure_aicommit_hooks_installed() -> bool {
  Path::new(".git/hooks/prepare-commit-msg").exists()
}

fn should_add_all() -> bool {
  env::args().any(|arg| arg == "--all")
}
