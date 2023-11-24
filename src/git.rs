#![allow(dead_code)]
#![allow(unused_imports)]

use git2::{
  Commit, Delta, Diff, DiffFormat, DiffOptions, Index, IndexAddOption, ObjectType, Oid, Repository, RepositoryInitOptions, RepositoryOpenFlags as Flag, StatusOptions, StatusShow
};
use anyhow::{anyhow, bail, Context, Result};
use log::{debug, error, info, trace, warn};
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use lazy_static::lazy_static;
use std::process::Command;
use std::path::Path;
use std::collections::HashSet;
use crate::chat;

// lazy_static! {
//   pub static ref REPO: Repo =
//     Repo::new().expect("Failed to initialize git repository");
// }

pub struct Repo {
  repo: Arc<RwLock<Repository>>
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
      repo: Arc::new(RwLock::new(repo))
    })
  }

  pub fn opts() -> DiffOptions {
    let mut opts = DiffOptions::new();
    opts
      //   .enable_fast_untracked_dirs(true)
      //   .ignore_whitespace_change(true)
      //   .recurse_untracked_dirs(false)
      //   .recurse_ignored_dirs(false)
      //   .ignore_whitespace_eol(true)
      //   .recurse_untracked_dirs(false)
      //   .ignore_blank_lines(true)
      //   .ignore_submodules(true)
      .include_untracked(false)
      .include_ignored(false);
    //   .interhunk_lines(0)
    //   .context_lines(0);
    opts
  }

  pub fn stats(&self) -> Result<git2::DiffStats> {
    let mut opts = Repo::opts();
    let repo = self.repo.read().expect("Failed to lock repo");
    let tree = repo
      .head()
      .context("Failed to get head")?
      .peel_to_tree()
      .context("Failed to peel head to tree")?;
    let index = repo.index().context("Failed to get index")?;

    // Pass the index explicitly to ensure that we are diffing against the staged changes only
    let diff =
      repo.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts))?;
    diff.stats().context("Failed to get diff stats")
  }

  pub fn diff(&self, max_token_count: usize) -> Result<String> {
    let mut opts = Repo::opts();
    let repo = self.repo.read().expect("Failed to lock repo");
    let mut pathspec = HashSet::new();
    let exclude_files: Option<Vec<String>> = None;

    // Include patterns to exclude files
    if let Some(ex_files) = exclude_files {
      for file in ex_files {
        opts.pathspec(format!(":(exclude){}", file));
        pathspec.insert(file.to_string());
      }
    }

    // Commonly excluded files
    let common_excludes = vec!["package-lock.json", "pnpm-lock.yaml", "*.lock"];
    for file in common_excludes {
      opts.pathspec(format!(":(exclude){}", file));
      pathspec.insert(file.to_string());
    }

    let mut opts = Repo::opts();
    let head = repo.head()?.peel_to_tree()?;
    let index = repo.index()?;
    let tree = repo
      .head()
      .context("Failed to get head")?
      .peel_to_tree()
      .context("Failed to peel head to tree")?;
    let diff =
      repo.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts))?;

    // Get names of staged files
    let mut files = Vec::new();
    diff.foreach(
      &mut |delta, _| {
        if let Some(file) = delta.new_file().path() {
          let file_path = file.to_string_lossy().into_owned();
          if !pathspec.contains(&file_path) {
            files.push(file_path);
          }
        }
        true
      },
      None,
      None,
      None
    )?;

    // Get the full diff
    let mut diff_str = Vec::new();
    diff.print(git2::DiffFormat::Patch, |_, _, line| {
      diff_str.extend_from_slice(line.content());
      true
    })?;

    let diff_output =
      String::from_utf8(diff_str).expect("Diff output is not valid UTF-8");

    Ok(diff_output)
  }

  pub fn diff2(&self, max_token_count: usize) -> Result<String> {
    let mut opts = Repo::opts();
    let repo = self.repo.read().expect("Failed to lock repo");
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

    let repo = self.repo.read().expect("Failed to lock repo");
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

pub fn repo() -> Repo {
  Repo::new().expect("Failed to initialize git repository")
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

    pub fn replace_file(&self, file_name: &str) {
      let random_content = Self::random_content();
      let file_path = self.path().join(file_name);
      let mut file = File::create(file_path).expect("Could not open file");
      file
        .write_all(random_content.as_bytes())
        .expect("Could not write to file");
    }

    pub fn create_file(&self, file_name: &str) {
      let random_content = Self::random_content();
      let file_path = self.path().join(file_name);
      let mut file = File::create(file_path).expect("Could not create file");
      file
        .write_all(random_content.as_bytes())
        .expect("Could not write to file");
    }

    pub fn delete_file(&self, file_name: &str) {
      let file_path = self.path().join(file_name);
      std::fs::remove_file(file_path).expect("Could not delete file");
    }

    fn stage_file(&self, file_name: &str) {
      let mut index = self.repo.index().expect("Could not get repo index");
      index
        .add_path(Path::new(file_name))
        .expect("Could not add file to index");
      index.write().expect("Could not write index");
      index.write_tree().expect("Could not write tree");
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

  fn setup() {
    _ = env_logger::builder().is_test(true).try_init();
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
    setup();

    let (helpers, repo) = Git2Helpers::new();

    /*  A file is created and committed */
    helpers.create_file("test.txt");
    helpers.stage_file("test.txt");
    helpers.commit();
    let stats = repo.stats().expect("Could not get diff stats");
    assert_eq!(stats.files_changed(), 0);
    assert_eq!(stats.insertions(), 0);
    assert_eq!(stats.deletions(), 0);

    /* Reset */
    helpers.commit();

    /* A new file is created and committed */
    helpers.create_file("other.txt");
    helpers.stage_file("other.txt");
    let stats = repo.stats().expect("Could not get diff stats");

    assert_eq!(stats.files_changed(), 1);
    assert_eq!(stats.insertions(), 1);
    assert_eq!(stats.deletions(), 0);

    /* Reset */
    helpers.commit();

    /* The file is modified and committed */
    helpers.replace_file("test.txt");
    helpers.stage_file("test.txt");
    let stats = repo.stats().expect("Could not get diff stats");
    assert_eq!(stats.files_changed(), 1);
    assert_eq!(stats.insertions(), 1);
    assert_eq!(stats.deletions(), 1);

    /* Reset */
    helpers.commit();

    /* The file is modified again without staging */
    helpers.create_file("new.txt");
    let stats = repo.stats().expect("Could not get diff stats");
    info!("Diff: {:?}", repo.diff(10000).expect("Could not generate diff"));
    assert_eq!(stats.files_changed(), 0);
    assert_eq!(stats.insertions(), 0);
    assert_eq!(stats.deletions(), 0);
  }
}
