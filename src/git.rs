#![allow(dead_code)]
#![allow(unused_imports)]

use git2::{
  Commit, Delta, DiffFormat, DiffOptions, Index, IndexAddOption, ObjectType, Oid, Repository, RepositoryInitOptions, RepositoryOpenFlags as Flag, StatusOptions, StatusShow
};
use anyhow::{anyhow, bail, Context, Result};
use log::{debug, error, info, trace, warn};
use std::sync::{LazyLock, Mutex};
use lazy_static::lazy_static;
use std::process::Command;
use std::path::Path;
use crate::chat;

lazy_static! {
  pub static ref REPO: Repo =
    Repo::new().expect("Failed to initialize git repository");
}

pub struct Repo {
  repo: Mutex<Repository>
}

impl Repo {
  pub fn new() -> Result<Self> {
    let repo = Repository::open_ext(".", Flag::empty(), Vec::<&Path>::new())
      .with_context(|| format!("Failed to open the git repository at"))?;

    Ok(Repo {
      repo: Mutex::new(repo)
    })
  }

  pub fn opts(&self) -> DiffOptions {
    let mut opts = DiffOptions::new();
    opts
      .enable_fast_untracked_dirs(true)
      .ignore_whitespace_change(true)
      .recurse_untracked_dirs(false)
      .recurse_ignored_dirs(false)
      .ignore_whitespace_eol(true)
      .recurse_untracked_dirs(false)
      .ignore_blank_lines(true)
      .ignore_submodules(true)
      .include_untracked(false)
      .include_ignored(false)
      .interhunk_lines(0)
      .context_lines(0)
      .minimal(true);
    opts
  }

  pub fn diff(
    &self, max_token_count: usize, repo: &Repository, index: Index
  ) -> Result<(String, Index)> {
    debug!("[diff] Generating diff with max token count: {}", max_token_count);

    let mut buf = Vec::new();
    let mut opts = self.opts();
    let mut count = 0;
    let tree = repo.head().context("Failed to get head")?.peel_to_tree()?;
    let diff = repo
      .diff_tree_to_index(Some(&tree), Some(&index), Some(&mut opts))
      .context("Failed to diff tree to index")?;

    diff
      .foreach(
        &mut |_file, _progress| true,
        None,
        None,
        Some(&mut |_delta, _hunk, line| {
          let content = line.content();
          let tokens: Vec<&[u8]> =
            content.split(|c| c.is_ascii_whitespace()).collect();
          let new_count = count + tokens.len();

          if new_count > max_token_count {
            return false;
          }

          buf.extend_from_slice(content);
          count = new_count;
          true
        })
      )
      .context("Failed to iterate over diff")?;

    if buf.is_empty() {
      bail!("Nothing to commit");
    }

    let str =
      String::from_utf8(buf).context("Failed to convert diff to string")?;
    Ok((str, index))
  }

  pub async fn commit(&self, add_all: bool) -> Result<()> {
    debug!("[commit] Committing with message");

    let repo = self.repo.lock().expect("Failed to lock repo");
    let mut index = repo.index().expect("Failed to get index");

    if add_all {
      debug!("Adding all files to index(--all)");

      index
        .add_all(["*"], IndexAddOption::DEFAULT, None)
        .context("Failed to add all files to index")?;
      index.write().context("Failed to write index")?;
    }

    let (diff, mut index) =
      self.diff(1000, &repo, index).context("Failed to generate diff")?;
    let oid = index.write_tree().context("Failed to write tree")?;
    let tree = repo.find_tree(oid).context("Failed to find tree")?;
    let signature = repo.signature().context("Failed to get signature")?;
    let parent = repo
      .head()
      .context("Failed to get head (2)")?
      .resolve()
      .context("Failed to resolve head")?
      .peel(ObjectType::Commit)
      .context("Failed to peel head")?
      .into_commit()
      .map_err(|_| anyhow!("Failed to resolve parent commit"))?;

    let message = chat::suggested_commit_message(diff)
      .await
      .context("Failed to generate commit message")?;

    repo
      .commit(Some("HEAD"), &signature, &signature, &message, &tree, &[&parent])
      .context("Failed to commit")?;

    Ok(())
  }
}

pub fn repo() -> &'static Repo {
  &REPO
}

#[cfg(test)]
mod tests {
  use git2::{Commit, IndexAddOption, ObjectType, Repository};
  use std::fs::File;
  use anyhow::{anyhow, bail, Context, Result};
  use std::io::Write;
  use std::path::Path;
  use tempfile::TempDir;

  pub struct Git2Helpers {
    pub repo: Repository,
    pub dir:  TempDir
  }

  impl Git2Helpers {
    pub fn new() -> Self {
      let dir = TempDir::new().expect("Could not create temp dir");
      let repo =
        Repository::init(dir.path()).expect("Could not initialize repo");
      Self {
        repo,
        dir
      }
    }

    pub fn create_file(&self, file_name: &str, content: &str) {
      let file_path = self.dir.path().join(file_name);
      let mut file = File::create(file_path).expect("Could not create file");
      writeln!(file, "{}", content).expect("Could not write to file");
      self.stage_file(file_name);
    }

    pub fn modify_file(&self, file_name: &str, content: &str) {
      let file_path = self.dir.path().join(file_name);
      let mut file = File::open(file_path).expect("Could not open file");
      writeln!(file, "{}", content).expect("Could not write to file");
      self.stage_file(file_name);
    }

    pub fn delete_file(&self, file_name: &str) {
      let file_path = self.dir.path().join(file_name);
      std::fs::remove_file(file_path).expect("Could not delete file");
      self.stage_file(file_name);
    }

    fn stage_file(&self, file_name: &str) {
      let mut index = self.repo.index().expect("Could not get repo index");
      index
        .add_path(Path::new(file_name))
        .expect("Could not add file to index");
      index.write().expect("Could not write index");
    }

    pub fn commit_changes(&self, message: &str) {
      let sig = self.repo.signature().expect("Could not create signature");
      let mut index = self.repo.index().expect("Could not get repo index");
      let oid = index.write_tree().expect("Could not write tree");
      let tree = self.repo.find_tree(oid).expect("Could not find tree");
      let mut parent_commits = Vec::new();
      if let Some(parent_commit) = self.find_last_commit() {
        parent_commits.push(parent_commit); // Add the parent commit directly
      }

      let parents: Vec<&Commit> = parent_commits.iter().collect();

      self
        .repo
        .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .expect("Could not commit");
    }

    fn find_last_commit(&self) -> Option<git2::Commit> {
      self
        .repo
        .head()
        .ok()
        .and_then(|ref_head| ref_head.resolve().ok())
        .and_then(|head| head.peel_to_commit().ok())
    }
  }
}

// **New File Addition**:
// 1. Create a new file in the repository.
// 2. Stage the new file with `git add`.
// 3. Commit the new file to the repository.
// 4. Add more content to the file without staging it.
// 5. Test `git diff` to ensure it shows the unstaged changes.

// **File Modification**:
// 1. Modify an existing file in the repository.
// 2. Stage the modifications.
// 3. Commit the changes.
// 4. Further modify the file without staging the changes.
// 5. Test `git diff` to ensure it shows the unstaged changes since the last commit.

// **File Deletion**:
// 1. Delete a file from the repository.
// 2. Stage the deletion with `git rm`.
// 3. Commit the deletion.
// 4. Test `git diff` with a previous commit to ensure it shows the file as deleted.

// **File Renaming**:
// 1. Rename an existing file in the repository.
// 2. Stage the rename with `git add` (for the new name) and `git rm` (for the old name).
// 3. Commit the changes.
// 4. Test `git diff` to ensure it shows the file as renamed.

// **Directory Changes**:
// 1. Make changes within a directory (add, modify, delete files).
// 2. Stage the directory changes.
// 3. Commit the directory changes.
// 4. Test `git diff` to ensure it shows all the changes made within the directory.

// **File Permissions Change**:
// 1. Change the permissions of a file without altering its content.
// 2. Stage the permission changes.
// 3. Commit the changes.
// 4. Test `git diff` to ensure it shows the permission changes.

// **Binary Files**:
// 1. Add or modify a binary file in the repository.
// 2. Stage and commit the binary file.
// 3. Modify the binary file again without staging.
// 4. Test `git diff` to ensure it indicates a binary file has changed.

// **Large Changes**:
// 1. Make a large number of changes to a file or multiple files.
// 2. Stage and commit these changes.
// 3. Make more large changes without staging.
// 4. Test `git diff` to ensure it shows all the large changes accurately.

// **Conflict Resolution**:
// 1. Create a merge conflict by editing the same part of a file in two different branches.
// 2. Attempt to merge the branches and observe the conflict.
// 3. Resolve the conflict manually.
// 4. Stage and commit the resolution.
// 5. Test `git diff` to ensure no differences are shown between the two branches now.

// **Submodule Updates**:
// 1. Update a submodule to a new commit.
// 2. Stage the submodule changes.
// 3. Commit the submodule changes.
// 4. Test `git diff` to ensure it shows the new commit reference for the submodule.

// **Line Endings**:
// 1. Change the line endings in a file from LF to CRLF or vice versa.
// 2. Stage and commit the line ending changes.
// 3. Test `git diff` with `--ignore-space-at-eol` to ensure it does/doesn't show the line ending changes based on the flag.

// **Whitespace Changes**:
// 1. Make whitespace changes in a file (add spaces or tabs).
// 2. Stage and commit these whitespace changes.
// 3. Test `git diff` with `--ignore-all-space` to ensure it doesn't show whitespace changes.

// **Empty Repository**:
// 1. Initialize an empty Git repository.
// 2. Test `git diff` to ensure it shows no output or changes.

// **Untracked Files**:
// 1. Add new files to the repository without staging them.
// 2. Test `git diff` to ensure it does not show untracked files.

// **Staged vs Unstaged**:
// 1. Make changes to a file and stage it.
// 2. Make additional changes to the same file without staging.
// 3. Test `git diff` and `git diff --staged` to ensure they show the correct staged and unstaged changes respectively.

// **Multiple File Changes**:
// 1. Make changes to multiple files in the repository.
// 2. Stage some files and leave others unstaged.
// 3. Commit the staged files.
// 4. Test `git diff` to ensure it shows only the changes in the unstaged files.

// **Branch Diffs**:
// 1. Create two branches and make different changes in each.
// 2. Commit the changes in each branch.
// 3. Test `git diff branch1..branch2` to ensure it shows the differences between the two branches.

// **Tag Diffs**:
// 1. Commit changes and tag the commit.
// 2. Make further changes and commit them.
// 3. Test `git diff tag..HEAD` to ensure it shows the changes made after the tag.

// **Undo Last Commit**:
// 1. Make and commit changes to a file.
// 2. Undo the commit with `git reset --soft HEAD~1`.
// 3. Test `git diff` to ensure it shows the changes that were in the undone commit.
