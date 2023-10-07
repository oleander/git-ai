use ansi_term::Colour::*;
use git2::{IndexAddOption, Repository, StatusOptions, StatusShow};
use std::env;
use std::path::Path;
use std::process::{exit, Command};

macro_rules! report {
  ($($arg:tt)*) => ({
    use std::io::Write;
    writeln!(&mut ::std::io::stderr(), $($arg)*).expect("Error writing to stderr");
    exit(1);
  })
}

fn main() {
  if !ensure_aicommit_hooks_installed() {
    report!("Error: aicommit hooks are not installed.");
  }

  if should_add_all() {
    if let Err(err) = run_git_add() {
      report!("Error adding changes to git: {}", err);
    }
  }

  let files_to_add = match get_git_status() {
    Ok(files) => files,
    Err(err) => report!("Error getting git status: {}", err),
  };

  if files_to_add.is_empty() {
    report!("No changes to commit");
  }

  match run_git_commit() {
    Ok(_) => {},
    Err(err) => report!("Error committing changes: {}", err),
  }

  let commit_message = match get_latest_commit_message() {
    Ok(message) => message,
    Err(err) => report!("Error getting latest commit message: {}", err),
  };

  println!("▶ {}", commit_message);

  for line in files_to_add {
    println!("   🔸{}", line);
  }
}

#[derive(PartialEq, Copy, Clone, Debug)]
enum GitStatus {
  A,
  M,
  D,
  U,
}

impl GitStatus {
  fn skipable(self) -> bool {
    self == GitStatus::U
  }

  fn colorized(self) -> ansi_term::ANSIString<'static> {
    match self {
      GitStatus::A => Green.paint("A"),
      GitStatus::M => Yellow.paint("M"),
      GitStatus::D => Red.paint("D"),
      GitStatus::U => White.paint("U"),
    }
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
      s if s.is_index_new() => GitStatus::A,
      s if s.is_index_modified() => GitStatus::M,
      s if s.is_index_deleted() => GitStatus::D,
      s if s.is_wt_new() => GitStatus::U,
      s if s.is_wt_deleted() => GitStatus::U,
      s if s.is_wt_modified() => GitStatus::U,
      _ => panic!("Unexpected git status: {:?}", entry.status()),
    };

    if status.skipable() {
      continue;
    }

    if let Some(path) = entry.path() {
      files.push(format!("{} {}", status.colorized(), path));
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
