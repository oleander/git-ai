use std::path::PathBuf;

use anyhow::Result;
use tempfile::TempDir;

pub struct TestRepo {
  pub repo:      git2::Repository,
  pub repo_path: TempDir
}

impl Default for TestRepo {
  fn default() -> Self {
    let repo_path = TempDir::new().unwrap();
    let repo = git2::Repository::init(repo_path.path()).unwrap();
    std::env::set_var("GIT_DIR", repo_path.path().join(".git"));

    Self {
      repo,
      repo_path
    }
  }
}

impl TestRepo {
  pub fn create_file(&self, name: &str, content: &str) -> Result<GitFile> {
    let file_path = self.repo_path.path().join(name);
    std::fs::write(&file_path, content)?;
    let repo = git2::Repository::open(self.repo.path()).unwrap();
    Ok(GitFile::new(repo, file_path, self.repo_path.path().to_path_buf()))
  }
}

pub struct GitFile {
  pub repo:      git2::Repository,
  pub path:      PathBuf,
  pub repo_path: PathBuf
}

impl GitFile {
  pub fn new(repo: git2::Repository, path: PathBuf, repo_path: PathBuf) -> Self {
    Self {
      repo,
      path,
      repo_path
    }
  }

  pub fn stage(&self) -> Result<()> {
    let mut index = self.repo.index()?;

    let relative_path = self.path.strip_prefix(&self.repo_path).unwrap();
    if !self.path.exists() {
      index.remove_path(relative_path)?;
      index.write()?;
    } else {
      index.add_path(relative_path)?;
      index.write()?;
    }

    Ok(())
  }

  pub fn commit(&self) -> Result<()> {
    let mut index = self.repo.index()?;
    let oid = index.write_tree()?;
    let signature = git2::Signature::now("Your Name", "email@example.com")?;
    let tree = self.repo.find_tree(oid)?;

    match self.find_last_commit() {
      Ok(parent_commit) => {
        self
          .repo
          .commit(Some("HEAD"), &signature, &signature, "Commit message", &tree, &[
            &parent_commit
          ])?;
      },
      Err(_) => {
        self
          .repo
          .commit(Some("HEAD"), &signature, &signature, "Initial commit", &tree, &[])?;
      }
    }

    Ok(())
  }

  pub fn delete(&self) -> Result<()> {
    std::fs::remove_file(&self.path)?;
    Ok(())
  }

  fn find_last_commit(&self) -> Result<git2::Commit, git2::Error> {
    let head = match self.repo.head() {
      Ok(head) => head,
      Err(e) => {
        if e.code() == git2::ErrorCode::UnbornBranch || e.code() == git2::ErrorCode::NotFound {
          return Err(e);
        } else {
          panic!("Failed to retrieve HEAD: {}", e);
        }
      }
    };

    let commit = head.peel_to_commit()?;
    Ok(commit)
  }
}
