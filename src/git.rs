#![allow(dead_code)]
#![allow(unused_imports)]

use git2::{
  Commit, Delta, Diff, DiffFormat, DiffOptions, Index, IndexAddOption, ObjectType, Oid, Repository, RepositoryInitOptions, RepositoryOpenFlags as Flag, StatusOptions, StatusShow
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
    Self::new_with_path(".".to_string())
  }

  pub fn new_with_path(path: String) -> Result<Self> {
    let repo =
      Repository::open_ext(path, Flag::empty(), Vec::<&Path>::new())
        .with_context(|| format!("Failed to open the git repository at"))?;

    Ok(Repo {
      repo: Mutex::new(repo)
    })
  }

  pub fn opts(&self) -> DiffOptions {
    let mut opts = DiffOptions::new();
    // opts
    //   .enable_fast_untracked_dirs(true)
    //   .ignore_whitespace_change(true)
    //   .recurse_untracked_dirs(false)
    //   .recurse_ignored_dirs(false)
    //   .ignore_whitespace_eol(true)
    //   .recurse_untracked_dirs(false)
    //   .ignore_blank_lines(true)
    //   .ignore_submodules(true)
    //   .include_untracked(false)
    //   .include_ignored(false)
    //   .interhunk_lines(0)
    //   .context_lines(0);
    opts
  }

  pub fn stats(&self) -> Result<git2::DiffStats> {
    let mut opts = self.opts();
    let repo = self.repo.lock().expect("Failed to lock repo");
    let tree = repo
      .head()
      .context("Failed to get head")?
      .peel_to_tree()
      .context("Failed to peel head to tree")?;
    let diff = repo.diff_tree_to_index(Some(&tree), None, Some(&mut opts))?;
    diff.stats().context("Failed to get diff stats")
  }

  pub fn diff(&self, max_token_count: usize) -> Result<String> {
    let mut opts = self.opts();
    let repo = self.repo.lock().expect("Failed to lock repo");
    let tree = repo
      .head()
      .context("Failed to get head")?
      .peel_to_tree()
      .context("Failed to peel head to tree")?;
    let diff = repo.diff_tree_to_index(Some(&tree), None, Some(&mut opts))?;

    let mut buf = Vec::new();
    let mut count = 0;

    // diff
    //   .foreach(
    //     &mut |_file, _progress| true,
    //     None,
    //     None,
    //     Some(&mut |_delta, _hunk, line| {
    //       let content = line.content();
    //       let tokens: Vec<&[u8]> =
    //         content.split(|c| c.is_ascii_whitespace()).collect();
    //       let new_count = count + tokens.len();

    //       if new_count > max_token_count {
    //         return false;
    //       }

    //       buf.extend_from_slice(content);
    //       count = new_count;
    //       true
    //     })
    //   )
    //   .context("Failed to iterate over diff")?;

    diff
      .print(DiffFormat::Patch, |_delta, _hunk, line| {
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
      .context("Failed to print diff")?;

    if buf.is_empty() {
      bail!("The diff is empty");
    }

    String::from_utf8(buf).context("Failed to convert diff to string")
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

    let diff = self.diff(1000).context("Failed to generate diff")?;
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
  use log::info;
  use anyhow::{anyhow, bail, Context, Result};
  use std::io::Write;
  use parsepatch::PatchReader;
  use std::path::Path;
  use tempfile::TempDir;
  use crate::git::Repo;

  pub struct Git2Helpers {
    pub repo: Repository,
    pub dir:  TempDir
  }

  impl Git2Helpers {
    pub fn new() -> (Self, Repo) {
      let dir = TempDir::new().expect("Could not create temp dir");
      let repo =
        Repository::init(dir.path()).expect("Could not initialize repo");
      let helper = Self {
        repo,
        dir
      };
      let repo2 = helper.into_repo();
      (helper, repo2)
    }

    pub fn path(&self) -> &Path {
      self.repo.path().parent().unwrap()
    }

    pub fn into_repo(&self) -> Repo {
      Repo::new_with_path(self.path().to_str().unwrap().to_string())
        .expect("Could not create repo")
    }

    fn random_content() -> String {
      std::iter::repeat(())
        .map(|()| rand::random::<char>())
        .filter(|c| c.is_ascii_alphanumeric())
        .take(5)
        .collect::<String>()
        + "\n"
    }

    pub fn replace_file(&self, file_name: &str) -> String {
      let random_content = Self::random_content();
      let file_path = self.path().join(file_name);
      let mut file = File::create(file_path).expect("Could not open file");
      file
        .write_all(random_content.as_bytes())
        .expect("Could not write to file");
      self.stage_file(file_name);

      random_content
    }

    pub fn create_file(&self, file_name: &str) -> String {
      let random_content = Self::random_content();
      let file_path = self.path().join(file_name);
      let mut file = File::create(file_path).expect("Could not create file");
      file
        .write_all(random_content.as_bytes())
        .expect("Could not write to file");
      self.stage_file(file_name);

      random_content
    }

    pub fn modify_file(&self, file_name: &str, content: &str) {
      let file_path = self.path().join(file_name);
      let mut file = File::create(file_path).expect("Could not open file");
      file.write_all(content.as_bytes()).expect("Could not write to file");
      self.stage_file(file_name);
    }

    pub fn delete_file(&self, file_name: &str) {
      let file_path = self.path().join(file_name);
      std::fs::remove_file(file_path).expect("Could not delete file");
      self.stage_deleted_file(file_name);
    }

    fn stage_file(&self, file_name: &str) {
      let mut index = self.repo.index().expect("Could not get repo index");
      index
        .add_path(Path::new(file_name))
        .expect("Could not add file to index");
      index.write().expect("Could not write index");
    }

    fn stage_deleted_file(&self, file_name: &str) {
      let mut index = self.repo.index().expect("Could not get repo index");
      index
        .remove_path(Path::new(file_name))
        .expect("Could not remove file from index");
      index.write().expect("Could not write index");
    }

    pub fn commit(&self) {
      let random_number = rand::random::<u8>();
      let message = format!("Commit {}", random_number);
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
        .commit(Some("HEAD"), &sig, &sig, message.as_str(), &tree, &parents)
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

  // **New File Addition**:
  // 1. Create a new file in the repository.
  // 2. Stage the new file with `git add`.
  // 3. Commit the new file to the repository.
  // 4. Add more content to the file without staging it.
  // 5. Test `git diff` to ensure it shows the unstaged changes.
  // #[test]
  // fn new_file_addition() {
  //   let (helpers, repo) = Git2Helpers::new();
  //   helpers.create_file("test.txt", "A\n");
  //   helpers.commit_changes("Initial commit");
  //   helpers.modify_file("test.txt", "A\nB\n");
  //   let (diff, _) = repo.diff(usize::MAX).expect("Could not generate diff");
  //   assert_eq!(diff, "\nB\n");
  // }

  // **File Modification**:
  // 1. Modify an existing file in the repository.
  // 2. Stage the modifications.
  // 3. Commit the changes.
  // 4. Further modify the file without staging the changes.
  // 5. Test `git diff` to ensure it shows the unstaged changes since the last commit.
  #[test]
  fn file_replacement() {
    let (helpers, repo) = Git2Helpers::new();

    helpers.create_file("test.txt");
    helpers.commit();

    let stats = repo.stats().expect("Could not get diff stats");
    assert_eq!(stats.files_changed(), 0);
    assert_eq!(stats.insertions(), 0);
    assert_eq!(stats.deletions(), 0);

    helpers.create_file("other.txt");

    let stats = repo.stats().expect("Could not get diff stats");
    assert_eq!(stats.files_changed(), 1);
    assert_eq!(stats.insertions(), 1);
    assert_eq!(stats.deletions(), 0);
    
    /* Reset */
    helpers.commit();

    helpers.replace_file("test.txt");
    let stats = repo.stats().expect("Could not get diff stats");
    assert_eq!(stats.files_changed(), 1);
    assert_eq!(stats.insertions(), 1);
    assert_eq!(stats.deletions(), 1);
  }

  // **File Deletion**:
  // 1. Delete a file from the repository.
  // 2. Stage the deletion with `git rm`.
  // 3. Commit the deletion.
  // 4. Test `git diff` with a previous commit to ensure it shows the file as deleted.
  // #[test]
  // fn file_deletion() {
  //   let (helpers, repo) = Git2Helpers::new();
  //   helpers.create_file("test.txt", "A\n");
  //   helpers.commit_changes("Initial commit");
  //   helpers.delete_file("test.txt");
  //   let (diff, _) = repo.diff(usize::MAX).expect("Could not generate diff");
  //   assert_eq!(diff, "-A\n");
  // }
}

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
