use git2::{IndexAddOption, Repository, StatusOptions, StatusShow};
use std::env;
use std::path::Path;
use std::process::{exit, Command};

fn main() {
  if !ensure_aicommit_hooks_installed() {
    eprintln!("Error: aicommit hooks are not installed.");
    exit(1);
  }

  let files_to_add = match get_git_status() {
    Ok(files) => files,
    Err(err) => {
      eprintln!("Error getting git status: {}", err);
      exit(1);
    },
  };

  if files_to_add.is_empty() {
    println!("No changes detected.");
    exit(0);
  }

  if should_add_all() {
    match run_git_add() {
      Ok(_) => {},
      Err(err) => {
        eprintln!("Error adding changes to git: {}", err);
        exit(1);
      },
    }
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

fn get_git_status() -> Result<Vec<String>, git2::Error> {
  let repo = Repository::open(".")?;
  let mut options = StatusOptions::new();
  options.show(StatusShow::IndexAndWorkdir);
  options.include_untracked(true);

  let statuses = repo.statuses(Some(&mut options))?;

  let mut files = Vec::new();
  for entry in statuses.iter().filter(|e| e.status() != git2::Status::CURRENT) {
    let status = match entry.status() {
      s if s.is_index_new() => "A",
      s if s.is_index_modified() => "M",
      s if s.is_index_deleted() => "D",
      s if s.is_wt_new() => "??",
      s if s.is_wt_modified() => "M",
      s if s.is_wt_deleted() => "D",
      _ => "",
    };

    if let Some(path) = entry.path() {
      files.push(format!("{} {}", status, path));
    }
  }

  Ok(files)
}

fn run_git_add() -> Result<(), git2::Error> {
  let repo = Repository::open(".")?;
  let mut index = repo.index()?;

  index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
  index.write()?;

  Ok(())
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
