use std::process::{Command, exit};
use ansi_term::Colour::{Green, Yellow};
use ansi_term::Colour::{Blue, Cyan};

fn main() {
    let files_to_add = get_git_status();

    if files_to_add.is_empty() {
        println!("No changes detected.");
        exit(0);
    }

    if !run_git_add() {
        eprintln!("Error adding changes to git.");
        exit(1);
    }

    if !run_git_commit() {
        eprintln!("Error committing changes.");
        exit(1);
    }

    let commit_message = get_latest_commit_message();

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

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(String::from)
        .collect()
}

fn run_git_add() -> bool {
    let output = Command::new("git")
        .arg("add")
        .arg(".")
        .output()
        .expect("Failed to execute git add");

    output.status.success()
}

fn run_git_commit() -> bool {
    let output = Command::new("git")
        .arg("commit")
        .arg("--no-edit")
        .output()
        .expect("Failed to execute git commit");

    output.status.success()
}

fn get_latest_commit_message() -> String {
    let output = Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--pretty=%B")
        .output()
        .expect("Failed to execute git log");

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
