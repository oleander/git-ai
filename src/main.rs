use std::process::{Command, exit};

fn main() {
    let color_green = "\x1b[32m";
    let color_yellow = "\x1b[33m";
    let color_reset = "\x1b[0m";

    let git_status_output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .output()
        .expect("Failed to execute git status")
        .stdout;

    let files_to_add = String::from_utf8_lossy(&git_status_output).to_string();

    if files_to_add.trim().is_empty() {
        println!("No changes detected.");
        exit(0);
    }

    let git_add_output = Command::new("git")
        .arg("add")
        .arg(".")
        .output()
        .expect("Failed to execute git add");

    if !git_add_output.status.success() {
        eprintln!(
            "Error adding changes to git: {}",
            String::from_utf8_lossy(&git_add_output.stderr)
        );
        exit(1);
    }

    let git_commit_output = Command::new("git")
        .arg("commit")
        .arg("--no-edit")
        .output()
        .expect("Failed to execute git commit");

    if !git_commit_output.status.success() {
        eprintln!(
            "Error committing changes: {}",
            String::from_utf8_lossy(&git_commit_output.stderr)
        );
        exit(1);
    }

    let commit_message = Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--pretty=%B")
        .output()
        .expect("Failed to execute git log")
        .stdout;

    let commit_message = String::from_utf8_lossy(&commit_message).trim().to_string();

    println!("{}{}{}:", color_green, commit_message, color_reset);

    for line in files_to_add.lines() {
        println!("  {}{}{}", color_yellow, line, color_reset);
    }
}
