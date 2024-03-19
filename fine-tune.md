# Examples

Examples of best practices for writing git commit messages:

## Example 0

### GIT DIFF:

diff --git c/src/bin/hook.rs c/src/bin/hook.rs
index c0c3d87..6358a87 100644
--- c/src/bin/hook.rs
+++ c/src/bin/hook.rs
@@ -2,0 +3,2 @@
use std::time::Duration;

@@ -7 +8,0 @@ use ai::{commit, config};
use std::time::Duration;

### COMMIT MESSAGE:

Add Duration import and remove duplicate in hook.rs

## Example 1

### GIT DIFF:

diff --git c/src/bin/fine-tune.rs c/src/bin/fine-tune.rs
index 72edc66..9535704 100644
--- c/src/bin/fine-tune.rs
+++ c/src/bin/fine-tune.rs
@@ -21,2 +21,2 @@ fn main() -> Result<()> {
  let config = repo.config().context("Couldn't access repository config")?;
  let user_email = config.get_string("user.email").context("Couldn't get user email")?;
  // let config = repo.config().context("Couldn't access repository config")?;
  // let user_email = config.get_string("user.email").context("Couldn't get user email")?;

### COMMIT MESSAGE:

Remove commented code related to config access in fine-tune.rs

## Example 2

### GIT DIFF:

diff --git c/src/bin/hook.rs c/src/bin/hook.rs
index 90328e4..c0c3d87 100644
--- c/src/bin/hook.rs
+++ c/src/bin/hook.rs
@@ -9 +8,0 @@ use ai::commit::Session;
use termion::event::Key;

### COMMIT MESSAGE:

Remove unused termion::event::Key import from hook.rs

## Example 3

### GIT DIFF:

diff --git c/src/bin/hook.rs c/src/bin/hook.rs
index eaa6260..90328e4 100644
--- c/src/bin/hook.rs
+++ c/src/bin/hook.rs
@@ -2,0 +3,4 @@
use indicatif::{ProgressBar, ProgressStyle};
use anyhow::{Context, Result};
use git2::{Oid, Repository};
use ai::{commit, config};
@@ -7,2 +9,0 @@ use termion::event::Key;
use git2::{Oid, Repository};
use anyhow::{Context, Result};
@@ -10,6 +10,0 @@ use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use tokio::time::sleep;
use tokio::signal;
use ai::{commit, config};
@@ -45 +40 @@ async fn main() -> Result<()> {
    // git ???
    // git rebase
@@ -61,0 +57 @@ async fn main() -> Result<()> {
  // Create a new session from the client
@@ -63,4 +59,9 @@ async fn main() -> Result<()> {
  let respomse = commit::generate(patch.to_string(), session.into(), pb.clone().into()).await?;
  let commit = respomse.response.trim();
  args.commit_msg_file.write(commit.trim().to_string()).unwrap();
  respomse.session.save_to_repo(&repo).await.unwrap();

  // If the user has a session, then we can use it to generate the commit message
  let response = commit::generate(patch.to_string(), session.into(), pb.clone().into()).await?;

  // Write the response to the commit message file
  args.commit_msg_file.write(response.response.trim().to_string()).unwrap();

  // Save the session to the repository
  response.session.save_to_repo(&repo).await.unwrap();

### COMMIT MESSAGE:

Reorganize imports and duplicate response handling in hook.rs

## Example 4

### GIT DIFF:

diff --git c/src/bin/hook.rs c/src/bin/hook.rs
index 6f0b378..eaa6260 100644
--- c/src/bin/hook.rs
+++ c/src/bin/hook.rs
@@ -54,0 +55,7 @@ async fn main() -> Result<()> {
  let pb_clone = pb.clone();
  ctrlc::set_handler(move || {
    pb_clone.finish_and_clear();
    console::Term::stdout().show_cursor().expect("Failed to show cursor");
    std::process::exit(1);
  })?;

@@ -65,24 +71,0 @@ async fn main() -> Result<()> {

async fn read_input(pb: ProgressBar) -> tokio::io::Result<i32> {
  let _stdout = std::io::stdout().into_raw_mode().unwrap();
  let mut stdin = termion::async_stdin().keys();

  loop {
    match stdin.next() {
      // Ctrl+C pressed: exit the program
      Some(Ok(Key::Ctrl('c'))) => {
        return Ok(1);
      },

      // Enter pressed: render empty line before progress bar
      Some(Ok(Key::Char('\n'))) => {
        pb.println("");
      },

      // Any other key pressed
      _ => {
        sleep(Duration::from_millis(50)).await;
      }
    }
  }
}

### COMMIT MESSAGE:

Implement Ctrl+C handler and remove read_input function in hook.rs

## Example 5

### GIT DIFF:

diff --git c/src/bin/hook.rs c/src/bin/hook.rs
index 13e0513..6f0b378 100644
--- c/src/bin/hook.rs
+++ c/src/bin/hook.rs
@@ -60,7 +59,0 @@ async fn main() -> Result<()> {
  log::debug!("Commit message generated successfully");
  let pb1 = pb.clone();
  tokio::select! {
    _ = signal::ctrl_c() => {
      console::Term::stdout().show_cursor().expect("Failed to show cursor");
      std::process::exit(1);
    }
@@ -68,7 +61 @@ async fn main() -> Result<()> {
    _ = read_input(pb1.clone()) => {
      pb1.finish_and_clear();
    }
  }

  log::debug!("Commit message generated successfully");
  pb1.finish_and_clear();
  pb.finish_and_clear();

### COMMIT MESSAGE:

Refactor async logic and cleanup in main function of hook.rs

## Example 6

### GIT DIFF:

diff --git c/src/bin/hook.rs c/src/bin/hook.rs
index 30206de..13e0513 100644
--- c/src/bin/hook.rs
+++ c/src/bin/hook.rs
@@ -60 +60 @@ async fn main() -> Result<()> {

  log::debug!("Commit message generated successfully");

### COMMIT MESSAGE:

Remove unnecessary newline in hook.rs

## Example 7

### GIT DIFF:

diff --git c/src/bin/hook.rs c/src/bin/hook.rs
index 03f757c..30206de 100644
--- c/src/bin/hook.rs
+++ c/src/bin/hook.rs
@@ -72,0 +73 @@ async fn main() -> Result<()> {
  log::debug!("Commit message generated successfully");

### COMMIT MESSAGE:

Add debug log for successful commit message generation in hook.rs

## Example 8

### GIT DIFF:

diff --git c/src/bin/hook.rs c/src/bin/hook.rs
index 3679066..03f757c 100644
--- c/src/bin/hook.rs
+++ c/src/bin/hook.rs
@@ -72,0 +73,2 @@ async fn main() -> Result<()> {
  pb1.finish_and_clear();


### COMMIT MESSAGE:

Add explicit progress bar finishing and clearing in hook.rs

## Example 9

### GIT DIFF:

diff --git c/src/bin/hook.rs c/src/bin/hook.rs
index f4b2e39..3679066 100644
--- c/src/bin/hook.rs
+++ c/src/bin/hook.rs
@@ -19,0 +20,2 @@ async fn main() -> Result<()> {
  env_logger::init();


### COMMIT MESSAGE:

Initialize env_logger in main function of hook.rs

## Example 10

### GIT DIFF:

diff --git c/src/commit.rs c/src/commit.rs
index 6d288f7..7b5b312 100644
--- c/src/commit.rs
+++ c/src/commit.rs
@@ -72,0 +73 @@ impl Session {
  // Load the session from the repository

### COMMIT MESSAGE:

Add comment explaining session loading from repo in commit.rs

## Example 11

### GIT DIFF:

diff --git c/src/commit.rs c/src/commit.rs
index 722e286..6d288f7 100644
--- c/src/commit.rs
+++ c/src/commit.rs
@@ -90,0 +91 @@ impl Session {
  // Save the session to the repository

### COMMIT MESSAGE:

Add comment explaining session saving to repo in commit.rs

## Example 12

### GIT DIFF:

diff --git c/src/install.rs c/src/install.rs
index 642c02b..04c3094 100644
--- c/src/install.rs
+++ c/src/install.rs
@@ -43,0 +44 @@ pub fn run() -> Result<(), InstallError> {
  // Check if the hook binary exists

### COMMIT MESSAGE:

Add hook binary existence check in install.rs

## Example 13

### GIT DIFF:

diff --git c/src/hook.rs c/src/hook.rs
index a327595..81f9268 100644
--- c/src/hook.rs
+++ c/src/hook.rs
@@ -93 +93,3 @@ impl PatchDiff for Diff<'_> {
      if *tokens + content.len() <= tokens_per_file {
      let curr_tokens = content.to_utf8().split_whitespace().count();
      if *tokens + curr_tokens < tokens_per_file {
        *tokens += curr_tokens;
@@ -95 +96,0 @@ impl PatchDiff for Diff<'_> {
        *tokens += content.to_utf8().split_whitespace().count();

### COMMIT MESSAGE:

Update hook.rs to count tokens and verify they stay within file limit

## Example 14

### GIT DIFF:

diff --git c/src/bin/fine-tune.rs c/src/bin/fine-tune.rs
index 58f3f1b..69cd2b2 100644
--- c/src/bin/fine-tune.rs
+++ c/src/bin/fine-tune.rs
diff --git c/src/config.rs c/src/config.rs
index 3ebe375..3f67bed 100644
--- c/src/config.rs
+++ c/src/config.rs
@@ -83,5 +83 @@ pub fn run(args: &ArgMatches) -> Result<()> {
      app.openai_api_key = args
        .get_one::<String>("<VALUE>")
        .context("Failed to parse openai-api-key")?
        .clone()
        .into();
      app.openai_api_key = args.get_one::<String>("<VALUE>").context("Failed to parse openai-api-key")?.clone().into();
diff --git c/src/examples.rs c/src/examples.rs
index f7b0f52..7323fc4 100644
--- c/src/examples.rs
+++ c/src/examples.rs
@@ -22,6 +22 @@ impl RepositoryExt for Repository {
    Ok(
      revwalk
        .take(max_commits)
        .map(move |id| self.find_commit(id.unwrap()).expect("Failed to find commit"))
        .collect()
    )
    Ok(revwalk.take(max_commits).map(move |id| self.find_commit(id.unwrap()).expect("Failed to find commit")).collect())
diff --git c/src/hook.rs c/src/hook.rs
index d84b194..a327595 100644
--- c/src/hook.rs
+++ c/src/hook.rs
@@ -44,6 +44 @@ impl DiffDeltaPath for git2::DiffDelta<'_> {
    self
      .new_file()
      .path()
      .or_else(|| self.old_file().path())
      .map(PathBuf::from)
      .unwrap_or_default()
    self.new_file().path().or_else(|| self.old_file().path()).map(PathBuf::from).unwrap_or_default()
@@ -141,3 +136 @@ impl<'a> PatchRepository for Repository {
    self
      .diff_tree_to_index(tree.as_ref(), None, Some(&mut opts))
      .context("Failed to get diff")
    self.diff_tree_to_index(tree.as_ref(), None, Some(&mut opts)).context("Failed to get diff")
diff --git c/src/main.rs c/src/main.rs
index 6e57fe9..8908627 100644
--- c/src/main.rs
+++ c/src/main.rs
@@ -25,16 +25,6 @@ fn cli() -> Command {
          .subcommand(
            Command::new("model").about("Sets the model to use").arg(
              Arg::new("<VALUE>")
                .required(true)
                .index(1)
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
            )
          )
          .subcommand(
            Command::new("language").about("Sets the language to use").arg(
              Arg::new("<VALUE>")
                .required(true)
                .index(1)
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
            )
          )
          .subcommand(Command::new("model").about("Sets the model to use").arg(
            Arg::new("<VALUE>").required(true).index(1).value_parser(clap::builder::NonEmptyStringValueParser::new())
          ))
          .subcommand(Command::new("language").about("Sets the language to use").arg(
            Arg::new("<VALUE>").required(true).index(1).value_parser(clap::builder::NonEmptyStringValueParser::new())
          ))
@@ -44,6 +34 @@ fn cli() -> Command {
              .arg(
                Arg::new("max-diff-tokens")
                  .required(true)
                  .index(1)
                  .value_parser(clap::value_parser!(usize))
              )
              .arg(Arg::new("max-diff-tokens").required(true).index(1).value_parser(clap::value_parser!(usize)))
@@ -56,8 +41,3 @@ fn cli() -> Command {
          .subcommand(
            Command::new("openai-api-key").about("Sets the OpenAI API key").arg(
              Arg::new("<VALUE>")
                .required(true)
                .index(1)
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
            )
          )
          .subcommand(Command::new("openai-api-key").about("Sets the OpenAI API key").arg(
            Arg::new("<VALUE>").required(true).index(1).value_parser(clap::builder::NonEmptyStringValueParser::new())
          ))
diff --git c/tests/common.rs c/tests/common.rs
index 465b80e..1d01f7f 100644
--- c/tests/common.rs
+++ c/tests/common.rs
@@ -71,3 +71 @@ impl GitFile {
        self
          .repo
          .commit(Some("HEAD"), &signature, &signature, "Commit message", &tree, &[&parent_commit])?;
        self.repo.commit(Some("HEAD"), &signature, &signature, "Commit message", &tree, &[&parent_commit])?;

### COMMIT MESSAGE:

Refactor code by combining operations into one-liners

## Example 15

### GIT DIFF:

diff --git c/src/hook.rs c/src/hook.rs
index 68bddf0..d84b194 100644
--- c/src/hook.rs
+++ c/src/hook.rs
@@ -9,0 +10 @@ use clap::Parser;
use tokio::io::AsyncReadExt;
@@ -99 +100 @@ impl PatchDiff for Diff<'_> {
        *tokens += content.len();
        *tokens += content.to_utf8().split_whitespace().count();

### COMMIT MESSAGE:

Update hook.rs to asynchronously read file and count tokens in content

## Example 16

### GIT DIFF:

diff --git c/src/bin/fine-tune.rs c/src/bin/fine-tune.rs
index 55578fa..872ca0a 100644
--- c/src/bin/fine-tune.rs
+++ c/src/bin/fine-tune.rs
@@ -68,0 +69,4 @@ fn main() -> Result<()> {
    if commit.contains("[") && commit.contains("]") {
      continue;
    }

@@ -71 +75 @@ fn main() -> Result<()> {
        { "role": "assistant", "content": commit, "weight": weight },
        { "role": "assistant", "content": commit.trim(), "weight": weight },

### COMMIT MESSAGE:

Ignore non rb files
## Example 17

### GIT DIFF:

diff --git c/src/bin/finetune.rs c/src/bin/finetune.rs
new file mode 100644
index 0000000..7c7213c
--- /dev/null
+++ c/src/bin/finetune.rs
@@ -0,0 +1,111 @@
use std::io::Write;
use std::fs::File;

use git2::{Commit, DiffFormat, DiffOptions, Repository};
use anyhow::{Context, Result};
use serde_json::json;

static PROMPT: &str = "Your role is to create concise git commit messages based on user-provided git diffs. When crafting these messages: - Focus on detailing the changes and reasons behind them, ensuring clarity and relevance. - Avoid including irrelevant or unnecessary details, such as translations, to maintain focus on the core changes. Your responses should be direct and immediately usable in a git commit, crafted in present tense to fit git conventions. You work primarily with git diffs, interpreting them to generate meaningful commit messages that succinctly summarize the changes.";

fn main() -> Result<()> {
  env_logger::init();

  let max_tokens = 16385;
  let file_name = "file-tune.json";
  let max_commits = 100;

  let mut opts = DiffOptions::new();
  opts
    .ignore_whitespace_change(true)
    .recurse_untracked_dirs(false)
    .recurse_ignored_dirs(false)
    .ignore_whitespace_eol(true)
    .ignore_blank_lines(true)
    .include_untracked(false)
    .ignore_whitespace(true)
    .indent_heuristic(false)
    .ignore_submodules(true)
    .include_ignored(false)
    .interhunk_lines(0)
    .context_lines(0)
    .patience(true)
    .minimal(true);

  log::info!("Creating fine-tune file with {} commits and {} tokens", max_commits, max_tokens);

  let repo = Repository::open(".").context("Failed to open git repository")?;
  let config = repo.config().context("Couldn't access repository config")?;
  let user_email = config.get_string("user.email").context("Couldn't get user email")?;
  let mut revwalk = repo.revwalk().context("Failed to create Revwalk")?;
  let mut file = File::create(file_name).context("Failed to create file")?;

  file.write_all(b"").context("Failed to write to file")?;

  revwalk.push_head().expect("Failed to push head");

  let mut curr_size = 0;
  let mut commit_count = 0;

  for oid in revwalk.take(max_commits) {
    let oid = oid.context("Failed to get oid")?;
    let commit = repo.find_commit(oid).context("Couldn't find commit")?;
    let commit = if commit.author().email() == Some(&user_email) {
      commit
    } else if commit.committer().email() == Some(&user_email) {
      commit
    } else {
      continue;
    };

    let Some(content) = generate_commit_diff(&repo, &commit, &opts) else {
      continue;
    };

    let Some(commit) = commit.message() else {
      continue;
    };

    let message = json!({
      "messages": [
        { "role": "assistant", "content": commit },
        { "role": "user", "content": content },
        { "role": "system", "content": PROMPT }
      ]
    });

    let content = serde_json::to_string_pretty(&message)?;
    curr_size += content.split_whitespace().count();

    if curr_size > max_tokens {
      log::warn!("Max tokens reached: {}", max_tokens);
      break;
    }

    commit_count += 1;
    file.write_all(content.as_bytes()).context("Failed to write to file")?;
  }

  log::info!("File {} created with {} commits", file_name, commit_count);

  Ok(())
}

fn generate_commit_diff(repo: &Repository, commit: &Commit, opts: &DiffOptions) -> Result<Option<String>> {
  let parent = commit.parents().next().unwrap_or_else(|| commit.clone());
  let tree = commit.tree().expect("Couldn't get commit tree");
  let parent_tree = parent.tree().expect("Couldn't get parent tree");

  let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut opts)).context("Failed to get diff")?;

  let mut patch: Vec<u8> = Vec::new();

  #[rustfmt::skip]
  diff.print(DiffFormat::Patch, |_, _, line| {
    let content = line.content();
    patch.extend_from_slice(content);
    true
  }).context("Failed to print diff")?;

  let content = String::from_utf8(patch).context("Failed to convert patch to string")?;
  if content.split_whitespace().count() > 500 { Ok(None) } else { Ok(Some(content)) }
}

### COMMIT MESSAGE:

Clean up
## Example 18

### GIT DIFF:

diff --git c/src/bin/finetune.rs c/src/bin/finetune.rs
index 36e1601..f1071d3 100644
--- c/src/bin/finetune.rs
+++ c/src/bin/finetune.rs
@@ -48,0 +49,4 @@ fn main() -> Result<()> {
    let Some(commit) = commit.message() else {
      continue;
    };

@@ -51,2 +55,2 @@ fn main() -> Result<()> {
        { "role": "user", "content": generate_commit_diff(&repo, &commit)? },
        { "role": "assistant", "content": commit.message().unwrap_or_default() },
        { "role": "assistant", "content": commit },
        { "role": "user", "content": content },
@@ -61 +63 @@ fn main() -> Result<()> {
    log::info!("Current size: {}", curr_size);

@@ -63 +65 @@ fn main() -> Result<()> {
      log::info!("Max tokens reached: {}", max_tokens);
      log::warn!("Max tokens reached: {}", max_tokens);
@@ -67 +68,0 @@ fn main() -> Result<()> {
    log::info!("Commit: {}", commit.id());

### COMMIT MESSAGE:

More clean up
