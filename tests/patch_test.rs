mod common;

use tempfile::NamedTempFile;
use anyhow::{Context, Result};
use git2::{DiffOptions, Repository, Tree};
use ai::hook::*;
use common::*;

#[test]
fn test_file_path_is_empty() {
  let named_file = NamedTempFile::new().unwrap();
  let path = named_file.path().to_path_buf();
  assert!(path.is_empty().unwrap());
}

#[test]
fn test_file_path_write_and_read() {
  let named_file = NamedTempFile::new().unwrap();
  let path = named_file.path().to_path_buf();
  let message = "Hello, world!";

  path.write(message.to_string()).unwrap();

  let contents = path.read().unwrap();

  assert_eq!(contents, message);
}

#[test]
fn test_utf8_string_to_utf8() {
  let bytes = vec![72, 101, 108, 108, 111];
  let utf8_string = bytes.to_utf8();

  assert_eq!(utf8_string, "Hello");
}

pub trait TestPatchDiff {
  fn test_is_empty(&self) -> Result<bool, anyhow::Error>;
  fn test_contains(&self, file: &GitFile) -> Result<bool, anyhow::Error>;
}

impl TestPatchDiff for git2::Diff<'_> {
  fn test_is_empty(&self) -> Result<bool, anyhow::Error> {
    let mut has_changes = false;

    self.foreach(
      &mut |_file, _progress| {
        has_changes = true;
        true
      },
      None,
      None,
      None
    )?;

    Ok(!has_changes)
  }

  fn test_contains(&self, our_file: &GitFile) -> Result<bool, anyhow::Error> {
    let mut found = false;
    let our_file_path = our_file.path.strip_prefix(&our_file.repo_path).unwrap();

    self.foreach(
      &mut |file, _progress| {
        let other_path = file.new_file().path().unwrap();
        if other_path == our_file_path {
          found = true;
        }

        true
      },
      None,
      None,
      None
    )?;

    Ok(found)
  }
}

trait TestRepository {
  fn to_diff(&self, tree: Option<Tree<'_>>) -> anyhow::Result<git2::Diff<'_>>;
}

impl TestRepository for Repository {
  fn to_diff(&self, tree: Option<Tree<'_>>) -> anyhow::Result<git2::Diff<'_>> {
    let mut opts = DiffOptions::new();
    opts
      .include_untracked(true)
      .recurse_untracked_dirs(true)
      .show_untracked_content(true);

    match tree {
      Some(tree) => {
        // For staged changes, compare tree to index
        let diff = self.diff_tree_to_index(Some(&tree), None, Some(&mut opts))?;
        if !diff.test_is_empty()? {
          return Ok(diff);
        }
        // If no staged changes, compare tree to workdir
        self.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts))
      }
      None => {
        // For initial state, compare HEAD to workdir
        match self.head() {
          Ok(head) => {
            let tree = head.peel_to_tree()?;
            self.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts))
          }
          Err(_) => {
            // No HEAD yet, show all files as new
            self.diff_tree_to_workdir(None, Some(&mut opts))
          }
        }
      }
    }
    .context("Failed to get diff")
  }
}

#[test]
fn test_empty_diff() {
  let repo = TestRepo::default();
  let file = repo.create_file("test.txt", "Hello, world!").unwrap();

  // Get initial diff before staging
  let repo_path = repo.repo_path.path().to_path_buf();
  let git_repo = git2::Repository::open(repo_path).unwrap();
  let diff = TestRepository::to_diff(&git_repo, None).unwrap();
  assert!(!TestPatchDiff::test_is_empty(&diff).unwrap());

  file.stage().unwrap();
  file.commit().unwrap();

  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
  let diff = TestRepository::to_diff(&git_repo, Some(tree.clone())).unwrap();
  assert!(TestPatchDiff::test_is_empty(&diff).unwrap());

  // Add a new line to the file
  let file = repo.create_file("file", "Hello, world!\n").unwrap();
  let diff = TestRepository::to_diff(&git_repo, Some(tree.clone())).unwrap();
  assert!(!TestPatchDiff::test_is_empty(&diff).unwrap());
  assert!(TestPatchDiff::test_contains(&diff, &file).unwrap());

  // stage and commit the file
  file.stage().unwrap();
  let diff = TestRepository::to_diff(&git_repo, Some(tree.clone())).unwrap();
  assert!(!TestPatchDiff::test_is_empty(&diff).unwrap());
  assert!(TestPatchDiff::test_contains(&diff, &file).unwrap());

  file.commit().unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
  let diff = TestRepository::to_diff(&git_repo, Some(tree.clone())).unwrap();
  assert!(TestPatchDiff::test_is_empty(&diff).unwrap());

  // delete the file
  file.delete().unwrap();
  let diff = TestRepository::to_diff(&git_repo, Some(tree.clone())).unwrap();
  assert!(!TestPatchDiff::test_is_empty(&diff).unwrap());
  assert!(TestPatchDiff::test_contains(&diff, &file).unwrap());

  // stage and commit the deletion
  file.stage().unwrap();
  let diff = TestRepository::to_diff(&git_repo, Some(tree.clone())).unwrap();
  assert!(!TestPatchDiff::test_is_empty(&diff).unwrap());
  assert!(TestPatchDiff::test_contains(&diff, &file).unwrap());

  file.commit().unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
  let diff = TestRepository::to_diff(&git_repo, Some(tree.clone())).unwrap();
  assert!(TestPatchDiff::test_is_empty(&diff).unwrap());

  // test initial commit
  let repo = TestRepo::default();
  let file = repo.create_file("test.txt", "Hello, world!").unwrap();
  let repo_path = repo.repo_path.path().to_path_buf();
  let git_repo = git2::Repository::open(repo_path).unwrap();
  let diff = TestRepository::to_diff(&git_repo, None).unwrap();
  assert!(!TestPatchDiff::test_is_empty(&diff).unwrap());
  assert!(TestPatchDiff::test_contains(&diff, &file).unwrap());

  // stage and commit the file
  file.stage().unwrap();
  let diff = TestRepository::to_diff(&git_repo, None).unwrap();
  assert!(!TestPatchDiff::test_is_empty(&diff).unwrap());
  assert!(TestPatchDiff::test_contains(&diff, &file).unwrap());

  file.commit().unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
  let diff = TestRepository::to_diff(&git_repo, Some(tree.clone())).unwrap();
  assert!(TestPatchDiff::test_is_empty(&diff).unwrap());
}

#[test]
fn test_diff_origin_characters() {
  // Test that origin characters (+, -, space) are properly included in diff output
  let repo = TestRepo::default();

  // Create initial file with multiple lines
  let file = repo
    .create_file("test.txt", "line 1\nline 2\nline 3\n")
    .unwrap();
  file.stage().unwrap();
  file.commit().unwrap();

  // Modify the file: remove line 2, keep line 1 and 3, add line 4
  let file = repo
    .create_file("test.txt", "line 1\nline 3\nline 4\n")
    .unwrap();
  file.stage().unwrap();

  // Get the diff
  let repo_path = repo.repo_path.path().to_path_buf();
  let git_repo = git2::Repository::open(&repo_path).unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
  let diff = TestRepository::to_diff(&git_repo, Some(tree)).unwrap();

  // Collect diff data
  use std::path::PathBuf;

  use ai::hook::PatchDiff;
  let diff_data = diff.collect_diff_data().unwrap();

  // Get the patch for our test file - use relative path
  let test_path = PathBuf::from("test.txt");
  let patch = diff_data.get(&test_path).expect("Should contain test.txt");

  // Verify that the patch contains origin characters
  // Should have lines starting with '+' for additions
  assert!(
    patch.lines().any(|line| line.starts_with("+line 4")),
    "Should contain '+line 4' for added line"
  );

  // Should have lines starting with '-' for deletions
  assert!(
    patch.lines().any(|line| line.starts_with("-line 2")),
    "Should contain '-line 2' for removed line"
  );

  // Should have lines starting with ' ' (space) for context
  assert!(
    patch
      .lines()
      .any(|line| line.starts_with(" line 1") || line.starts_with(" line 3")),
    "Should contain context lines starting with space"
  );
}

#[test]
fn test_diff_only_additions() {
  // Test a diff with only additions (new file)
  let repo = TestRepo::default();
  let file = repo
    .create_file("new_file.txt", "new line 1\nnew line 2\n")
    .unwrap();
  file.stage().unwrap();

  let repo_path = repo.repo_path.path().to_path_buf();
  let git_repo = git2::Repository::open(&repo_path).unwrap();
  let diff = TestRepository::to_diff(&git_repo, None).unwrap();

  use std::path::PathBuf;

  use ai::hook::PatchDiff;
  let diff_data = diff.collect_diff_data().unwrap();
  let new_file_path = PathBuf::from("new_file.txt");
  let patch = diff_data
    .get(&new_file_path)
    .expect("Should contain new_file.txt");

  // All content lines should start with '+'
  assert!(patch.lines().any(|line| line.starts_with("+new line 1")), "Should contain '+new line 1'");
  assert!(patch.lines().any(|line| line.starts_with("+new line 2")), "Should contain '+new line 2'");
}

#[test]
fn test_diff_only_deletions() {
  // Test a diff with only deletions (deleted file)
  let repo = TestRepo::default();
  let file = repo
    .create_file("to_delete.txt", "delete line 1\ndelete line 2\n")
    .unwrap();
  file.stage().unwrap();
  file.commit().unwrap();

  // Delete the file
  file.delete().unwrap();
  file.stage().unwrap();

  let repo_path = repo.repo_path.path().to_path_buf();
  let git_repo = git2::Repository::open(&repo_path).unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
  let diff = TestRepository::to_diff(&git_repo, Some(tree)).unwrap();

  use std::path::PathBuf;

  use ai::hook::PatchDiff;
  let diff_data = diff.collect_diff_data().unwrap();
  let delete_path = PathBuf::from("to_delete.txt");
  let patch = diff_data
    .get(&delete_path)
    .expect("Should contain to_delete.txt");

  // All content lines should start with '-'
  assert!(
    patch.lines().any(|line| line.starts_with("-delete line 1")),
    "Should contain '-delete line 1'"
  );
  assert!(
    patch.lines().any(|line| line.starts_with("-delete line 2")),
    "Should contain '-delete line 2'"
  );
}

#[test]
fn test_collect_diff_data_includes_headers() {
  // Regression test: collect_diff_data() must include file and hunk headers
  // so that parse_diff() can split the output into per-file sections.
  // Previously, headers were stripped, causing all commits to be treated
  // as a single "unknown" file in the multi-step analysis pipeline.
  let repo = TestRepo::default();

  let file = repo.create_file("auth.rs", "fn login() {}\n").unwrap();
  file.stage().unwrap();
  file.commit().unwrap();

  // Modify the file
  let file = repo
    .create_file("auth.rs", "fn login() { validate(); }\n")
    .unwrap();
  file.stage().unwrap();

  let repo_path = repo.repo_path.path().to_path_buf();
  let git_repo = git2::Repository::open(&repo_path).unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
  let diff = TestRepository::to_diff(&git_repo, Some(tree)).unwrap();

  use std::path::PathBuf;

  use ai::hook::PatchDiff;
  let diff_data = diff.collect_diff_data().unwrap();
  let path = PathBuf::from("auth.rs");
  let patch = diff_data.get(&path).expect("Should contain auth.rs");

  // Must contain diff header for parse_diff() to work
  assert!(
    patch.contains("diff --git"),
    "collect_diff_data() must include file headers (diff --git), got:\n{patch}"
  );

  // Must contain hunk header
  assert!(patch.contains("@@"), "collect_diff_data() must include hunk headers (@@), got:\n{patch}");

  // Must contain a real added *content* line — not just the `+++` file header.
  // (A bare `starts_with('+')` check would be satisfied by `+++ b/auth.rs`.)
  assert!(
    patch
      .lines()
      .any(|l| l.starts_with('+') && !l.starts_with("+++") && l.contains("validate()")),
    "Should contain the added content line (+...validate()...), got:\n{patch}"
  );
}

#[test]
fn test_collect_diff_data_multi_file_headers() {
  // Verify that each file in a multi-file diff gets its own headers,
  // enabling parse_diff() to split them correctly.
  let repo = TestRepo::default();

  let f1 = repo.create_file("file_a.rs", "fn a() {}\n").unwrap();
  f1.stage().unwrap();
  let f2 = repo.create_file("file_b.rs", "fn b() {}\n").unwrap();
  f2.stage().unwrap();
  f1.commit().unwrap();

  // Modify both files
  let f1 = repo.create_file("file_a.rs", "fn a() { 1 }\n").unwrap();
  f1.stage().unwrap();
  let f2 = repo.create_file("file_b.rs", "fn b() { 2 }\n").unwrap();
  f2.stage().unwrap();

  let repo_path = repo.repo_path.path().to_path_buf();
  let git_repo = git2::Repository::open(&repo_path).unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
  let diff = TestRepository::to_diff(&git_repo, Some(tree)).unwrap();

  use std::path::PathBuf;

  use ai::hook::PatchDiff;
  let diff_data = diff.collect_diff_data().unwrap();

  // Both files should have their own diff headers
  let pa = PathBuf::from("file_a.rs");
  let pb = PathBuf::from("file_b.rs");
  let patch_a = diff_data.get(&pa).expect("Should contain file_a.rs");
  let patch_b = diff_data.get(&pb).expect("Should contain file_b.rs");

  assert!(patch_a.contains("diff --git"), "file_a.rs should have its own diff header");
  assert!(patch_b.contains("diff --git"), "file_b.rs should have its own diff header");
}

#[test]
fn test_to_patch_is_deterministic_across_runs() {
  // C4: Identical staged input must produce a byte-for-byte identical patch every time.
  // Previously the patch's file order followed HashMap iteration / thread-completion order,
  // so the same staged tree could yield different patches and thus different commit messages.
  let repo = TestRepo::default();

  // Several files so multi-file ordering is meaningfully exercised. Names are intentionally
  // out of alphabetical insertion order to catch ordering that depends on insertion.
  let names = ["zeta.rs", "alpha.rs", "mid.rs", "beta.rs", "gamma.rs", "delta.rs"];
  for name in names {
    let f = repo
      .create_file(name, &format!("// initial {name}\nfn {}() {{}}\n", name.replace('.', "_")))
      .unwrap();
    f.stage().unwrap();
  }
  // Commit the initial versions via the last file handle's commit.
  let first = repo
    .create_file(names[0], &format!("// initial {}\nfn zeta_rs() {{}}\n", names[0]))
    .unwrap();
  first.stage().unwrap();
  first.commit().unwrap();

  // Modify every file so each appears in the diff.
  for name in names {
    let f = repo
      .create_file(name, &format!("// changed {name}\nfn {}() {{ 1 + 1; }}\n", name.replace('.', "_")))
      .unwrap();
    f.stage().unwrap();
  }

  let repo_path = repo.repo_path.path().to_path_buf();

  use ai::hook::PatchRepository;
  use ai::model::Model;

  // Generate the patch many times, each via a freshly opened repo (fresh HashMap each time).
  let mut patches = Vec::new();
  for _ in 0..10 {
    let git_repo = git2::Repository::open(&repo_path).unwrap();
    let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
    let patch = git_repo
      .to_patch(Some(tree), 4096, Model::default())
      .unwrap();
    patches.push(patch);
  }

  // All runs must be byte-identical.
  let first_patch = &patches[0];
  for (i, p) in patches.iter().enumerate() {
    assert_eq!(p, first_patch, "patch run {i} differs from run 0 — output is non-deterministic");
  }

  // The patch must contain all files and parse into the expected set (order may follow the
  // token-packing sort, but it is stable: the byte-identical assertion above proves the
  // ordering itself is deterministic across runs).
  use ai::multi_step_integration::parse_diff;
  let parsed = parse_diff(first_patch).unwrap();
  let mut paths: Vec<String> = parsed.iter().map(|f| f.path.clone()).collect();
  paths.sort();
  let mut expected: Vec<String> = names.iter().map(|s| s.to_string()).collect();
  expected.sort();
  assert_eq!(paths, expected, "patch should contain exactly the staged files, got {paths:?}");
}

/// Stages `count` modified files (each with `body_lines` of changed content) and returns the
/// repo path. The files go through an initial commit then a modification, so all appear in the
/// diff against HEAD. Used to drive the >20-file *parallel* path in `to_patch`.
fn build_many_file_repo(count: usize, body_lines: usize) -> (TestRepo, std::path::PathBuf) {
  let repo = TestRepo::default();

  let make_body = |name: &str, marker: &str| {
    let mut s = format!("// {marker} {name}\n");
    for i in 0..body_lines {
      s.push_str(&format!("fn {}_{i}() {{ let x = {i}; let y = x + 1; }}\n", name.replace('.', "_")));
    }
    s
  };

  // Initial versions, committed.
  for n in 0..count {
    let name = format!("file_{n:03}.rs");
    let f = repo
      .create_file(&name, &make_body(&name, "initial"))
      .unwrap();
    f.stage().unwrap();
  }
  let first = repo
    .create_file("file_000.rs", &make_body("file_000.rs", "initial"))
    .unwrap();
  first.stage().unwrap();
  first.commit().unwrap();

  // Modify every file so each shows up in the diff.
  for n in 0..count {
    let name = format!("file_{n:03}.rs");
    let f = repo
      .create_file(&name, &make_body(&name, "changed"))
      .unwrap();
    f.stage().unwrap();
  }

  let path = repo.repo_path.path().to_path_buf();
  (repo, path)
}

#[test]
fn test_to_patch_parallel_path_deterministic_no_truncation() {
  // C4 / C3b: drives the >20-file PARALLEL path. With a generous token budget no truncation
  // happens, so this isolates output-ordering determinism in the parallel branch.
  let (_repo, repo_path) = build_many_file_repo(30, 3);

  use ai::hook::PatchRepository;
  use ai::model::Model;

  let mut patches = Vec::new();
  for _ in 0..10 {
    let git_repo = git2::Repository::open(&repo_path).unwrap();
    let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
    let patch = git_repo
      .to_patch(Some(tree), 100_000, Model::default())
      .unwrap();
    patches.push(patch);
  }

  let first_patch = &patches[0];
  assert!(!first_patch.is_empty(), "parallel-path patch should not be empty");
  for (i, p) in patches.iter().enumerate() {
    assert_eq!(p, first_patch, "parallel-path patch run {i} differs — non-deterministic output");
  }
}

#[test]
fn test_to_patch_parallel_path_deterministic_with_truncation() {
  // C4 / C3b: drives the >20-file PARALLEL path with a SMALL budget so total content far
  // exceeds max_tokens and per-file truncation actually triggers. This is exactly the scenario
  // where the old shared-atomic token budget raced across threads and produced non-deterministic
  // bytes; the sequential-budget + order-preserving rewrite must yield byte-identical output.
  let (_repo, repo_path) = build_many_file_repo(40, 12);

  use ai::hook::PatchRepository;
  use ai::model::Model;

  let mut patches = Vec::new();
  for _ in 0..10 {
    let git_repo = git2::Repository::open(&repo_path).unwrap();
    let tree = git_repo.head().unwrap().peel_to_tree().unwrap();
    // Small budget relative to the ~40 files * 12 lines of content => forces truncation.
    let patch = git_repo
      .to_patch(Some(tree), 300, Model::default())
      .unwrap();
    patches.push(patch);
  }

  let first_patch = &patches[0];
  for (i, p) in patches.iter().enumerate() {
    assert_eq!(
      p, first_patch,
      "parallel-path patch with truncation run {i} differs — truncation/order is non-deterministic"
    );
  }
}

#[test]
fn test_to_patch_output_parseable_by_parse_diff() {
  // End-to-end test: the output of to_patch() must be parseable by
  // parse_diff() into correct per-file sections. This tests the full
  // pipeline that was previously broken.
  let repo = TestRepo::default();

  let f1 = repo.create_file("main.rs", "fn main() {}\n").unwrap();
  f1.stage().unwrap();
  let f2 = repo.create_file("lib.rs", "pub fn hello() {}\n").unwrap();
  f2.stage().unwrap();
  f1.commit().unwrap();

  // Modify both files
  let f1 = repo
    .create_file("main.rs", "fn main() { hello(); }\n")
    .unwrap();
  f1.stage().unwrap();
  let f2 = repo
    .create_file("lib.rs", "pub fn hello() { println!(\"hi\"); }\n")
    .unwrap();
  f2.stage().unwrap();

  let repo_path = repo.repo_path.path().to_path_buf();
  let git_repo = git2::Repository::open(&repo_path).unwrap();
  let tree = git_repo.head().unwrap().peel_to_tree().unwrap();

  // Generate patch via the same path the hook uses
  use ai::hook::PatchRepository;
  let patch = git_repo
    .to_patch(Some(tree), 4096, ai::model::Model::default())
    .unwrap();

  // Now parse it the same way multi_step_integration does
  use ai::multi_step_integration::parse_diff;
  let parsed = parse_diff(&patch).unwrap();

  // Must produce 2 files with correct paths — NOT a single "unknown" file
  assert!(
    parsed.len() == 2,
    "parse_diff should find 2 files, got {} file(s): {:?}",
    parsed.len(),
    parsed.iter().map(|f| &f.path).collect::<Vec<_>>()
  );

  let paths: Vec<&str> = parsed.iter().map(|f| f.path.as_str()).collect();
  assert!(paths.contains(&"main.rs"), "Should contain main.rs, got: {paths:?}");
  assert!(paths.contains(&"lib.rs"), "Should contain lib.rs, got: {paths:?}");

  // None should be "unknown" (the old broken fallback)
  assert!(!paths.contains(&"unknown"), "Should NOT fall back to 'unknown' file, got: {paths:?}");
}
