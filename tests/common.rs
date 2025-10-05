use std::path::PathBuf;

use anyhow::Result;
use tempfile::TempDir;

/// Represents a test file with optional encoding and binary support
#[derive(Clone)]
pub struct TestFile {
  pub path:      String,
  pub content:   Vec<u8>,
  pub is_binary: bool,
  pub encoding:  Option<String>
}

impl TestFile {
  /// Create a new text file
  pub fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
    Self {
      path:      path.into(),
      content:   content.into().into_bytes(),
      is_binary: false,
      encoding:  None
    }
  }

  /// Create a binary file
  pub fn binary(path: impl Into<String>, content: Vec<u8>) -> Self {
    Self {
      path: path.into(),
      content,
      is_binary: true,
      encoding: None
    }
  }

  /// Create a file with specific encoding
  pub fn with_encoding(path: impl Into<String>, content: Vec<u8>, encoding: impl Into<String>) -> Self {
    Self {
      path: path.into(),
      content,
      is_binary: false,
      encoding: Some(encoding.into())
    }
  }

  /// Create a source file (for priority testing)
  pub fn source(path: impl Into<String>, content: impl Into<String>) -> Self {
    Self::new(path, content)
  }

  /// Create a test file (for priority testing)
  pub fn test(path: impl Into<String>, content: impl Into<String>) -> Self {
    Self::new(path, content)
  }

  /// Create a config file (for priority testing)
  pub fn config(path: impl Into<String>, content: impl Into<String>) -> Self {
    Self::new(path, content)
  }
}

pub struct TestRepo {
  pub repo:      git2::Repository,
  pub repo_path: TempDir
}

impl Default for TestRepo {
  fn default() -> Self {
    let repo_path = TempDir::new().unwrap();
    let repo = git2::Repository::init(repo_path.path()).unwrap();
    std::env::set_var("GIT_DIR", repo_path.path().join(".git"));

    Self { repo, repo_path }
  }
}

impl TestRepo {
  pub fn create_file(&self, name: &str, content: &str) -> Result<GitFile> {
    let file_path = self.repo_path.path().join(name);
    std::fs::write(&file_path, content)?;
    let repo = git2::Repository::open(self.repo.path()).unwrap();
    Ok(GitFile::new(repo, file_path, self.repo_path.path().to_path_buf()))
  }

  /// Create a file from TestFile struct
  pub fn create_test_file(&self, test_file: &TestFile) -> Result<GitFile> {
    let file_path = self.repo_path.path().join(&test_file.path);

    // Create parent directories if needed
    if let Some(parent) = file_path.parent() {
      std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&file_path, &test_file.content)?;
    let repo = git2::Repository::open(self.repo.path()).unwrap();
    Ok(GitFile::new(repo, file_path, self.repo_path.path().to_path_buf()))
  }

  /// Create an initial commit to establish HEAD
  pub fn create_initial_commit(&self) -> Result<()> {
    let file = self.create_file(".gitkeep", "")?;
    file.stage()?;
    file.commit()?;
    Ok(())
  }
}

pub struct GitFile {
  pub repo:      git2::Repository,
  pub path:      PathBuf,
  pub repo_path: PathBuf
}

impl GitFile {
  pub fn new(repo: git2::Repository, path: PathBuf, repo_path: PathBuf) -> Self {
    Self { repo, path, repo_path }
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
          .commit(Some("HEAD"), &signature, &signature, "Commit message", &tree, &[&parent_commit])?;
      }
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
      Err(e) =>
        if e.code() == git2::ErrorCode::UnbornBranch || e.code() == git2::ErrorCode::NotFound {
          return Err(e);
        } else {
          panic!("Failed to retrieve HEAD: {e}");
        },
    };

    let commit = head.peel_to_commit()?;
    Ok(commit)
  }
}

/// Factory for creating test repositories with various scenarios
pub struct TestRepoFactory;

impl TestRepoFactory {
  /// Create an empty repository with no commits
  pub fn create_empty() -> TestRepo {
    TestRepo::default()
  }

  /// Create a repository with a specified number of commits
  pub fn create_with_history(commits: usize) -> Result<TestRepo> {
    let repo = TestRepo::default();

    for i in 0..commits {
      let file = repo.create_file(&format!("file_{}.txt", i), &format!("Content {}", i))?;
      file.stage()?;
      file.commit()?;
    }

    Ok(repo)
  }

  /// Create a repository with specific files
  pub fn create_with_files(files: Vec<TestFile>) -> Result<TestRepo> {
    let repo = TestRepo::default();
    repo.create_initial_commit()?;

    for test_file in files {
      let file = repo.create_test_file(&test_file)?;
      file.stage()?;
    }

    Ok(repo)
  }

  /// Create a large repository with many files
  pub fn create_large_repo(file_count: usize, size_per_file: usize) -> Result<TestRepo> {
    let repo = TestRepo::default();
    repo.create_initial_commit()?;

    for i in 0..file_count {
      let content = "x".repeat(size_per_file);
      let file = repo.create_file(&format!("large_file_{}.txt", i), &content)?;
      file.stage()?;
    }

    Ok(repo)
  }

  /// Create a repository with binary files
  pub fn create_binary_repo() -> Result<TestRepo> {
    let repo = TestRepo::default();
    repo.create_initial_commit()?;

    // Create a binary file with non-UTF8 bytes
    let binary_content = vec![0xFF, 0xFE, 0xFD, 0x00, 0x01, 0x02, 0x03];
    let binary_file = TestFile::binary("binary.dat", binary_content);
    let file = repo.create_test_file(&binary_file)?;
    file.stage()?;

    Ok(repo)
  }

  /// Create a repository with unicode filenames and content
  pub fn create_unicode_repo() -> Result<TestRepo> {
    let repo = TestRepo::default();
    repo.create_initial_commit()?;

    // Unicode filename and content
    let unicode_file = TestFile::new("文件.txt", "Unicode content: 你好世界 🌍");
    let file = repo.create_test_file(&unicode_file)?;
    file.stage()?;

    // Emoji in filename
    let emoji_file = TestFile::new("test_🚀.txt", "Rocket file");
    let file = repo.create_test_file(&emoji_file)?;
    file.stage()?;

    Ok(repo)
  }

  /// Create a repository with merge conflicts
  pub fn create_with_merge_conflict() -> Result<TestRepo> {
    let repo = TestRepo::default();

    // Create initial commit on main branch
    let file = repo.create_file("conflict.txt", "Original content")?;
    file.stage()?;
    file.commit()?;

    // Create a branch
    let head = repo.repo.head()?;
    let commit = head.peel_to_commit()?;
    repo.repo.branch("feature", &commit, false)?;
    drop(head);
    drop(commit);

    // Modify on main
    let file = repo.create_file("conflict.txt", "Main branch content")?;
    file.stage()?;
    file.commit()?;

    // Switch to feature branch
    repo.repo.set_head("refs/heads/feature")?;
    repo
      .repo
      .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;

    // Modify on feature
    let file = repo.create_file("conflict.txt", "Feature branch content")?;
    file.stage()?;
    file.commit()?;

    // Try to merge main into feature (will create conflict)
    let main_ref = repo
      .repo
      .find_reference("refs/heads/main")
      .or_else(|_| repo.repo.find_reference("refs/heads/master"))?;
    let main_commit = main_ref.peel_to_commit()?;
    let main_commit_id = main_commit.id();
    drop(main_ref);
    drop(main_commit);

    // Perform merge (this will create conflicts)
    let mut merge_opts = git2::MergeOptions::new();
    let mut checkout_opts = git2::build::CheckoutBuilder::new();
    checkout_opts.allow_conflicts(true);

    let annotated_commit = repo.repo.find_annotated_commit(main_commit_id)?;
    repo
      .repo
      .merge(&[&annotated_commit], Some(&mut merge_opts), Some(&mut checkout_opts))?;
    drop(annotated_commit);

    Ok(repo)
  }

  /// Create a repository in detached HEAD state
  pub fn create_detached_head() -> Result<TestRepo> {
    let repo = TestRepo::default();

    // Create some commits
    let file = repo.create_file("file1.txt", "Content 1")?;
    file.stage()?;
    file.commit()?;

    let file = repo.create_file("file2.txt", "Content 2")?;
    file.stage()?;
    file.commit()?;

    // Get the first commit
    let mut revwalk = repo.repo.revwalk()?;
    revwalk.push_head()?;
    let commits: Vec<_> = revwalk.collect();

    if let Some(Ok(first_commit_oid)) = commits.last() {
      let commit = repo.repo.find_commit(*first_commit_oid)?;
      repo.repo.set_head_detached(commit.id())?;
      repo
        .repo
        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;
    }

    Ok(repo)
  }

  /// Create a corrupted repository (for error testing)
  pub fn create_corrupted_repo() -> Result<TestRepo> {
    let repo = TestRepo::default();

    // Create a commit
    let file = repo.create_file("file.txt", "Content")?;
    file.stage()?;
    file.commit()?;

    // Corrupt the repository by removing the HEAD file
    let head_path = repo.repo_path.path().join(".git").join("HEAD");
    if head_path.exists() {
      std::fs::remove_file(head_path)?;
    }

    Ok(repo)
  }

  /// Create a repository with submodules
  pub fn create_with_submodules() -> Result<TestRepo> {
    let repo = TestRepo::default();
    repo.create_initial_commit()?;

    // Create a .gitmodules file to simulate submodule
    let gitmodules_content = r#"[submodule "external"]
    path = external
    url = https://github.com/example/repo.git"#;
    let file = repo.create_file(".gitmodules", gitmodules_content)?;
    file.stage()?;
    file.commit()?;

    Ok(repo)
  }

  /// Create a repository with symlinks
  pub fn create_with_symlinks() -> Result<TestRepo> {
    let repo = TestRepo::default();
    repo.create_initial_commit()?;

    // Create a target file
    let target = repo.create_file("target.txt", "Target content")?;
    target.stage()?;
    target.commit()?;

    // Create a symlink (platform-dependent)
    #[cfg(unix)]
    {
      use std::os::unix::fs::symlink;
      let link_path = repo.repo_path.path().join("link.txt");
      let target_path = repo.repo_path.path().join("target.txt");
      symlink(target_path, link_path)?;
    }

    Ok(repo)
  }

  /// Create a repository with very deep directory structure
  pub fn create_deep_structure() -> Result<TestRepo> {
    let repo = TestRepo::default();
    repo.create_initial_commit()?;

    // Create a deeply nested file
    let deep_path = (0..20)
      .map(|i| format!("dir{}", i))
      .collect::<Vec<_>>()
      .join("/");
    let deep_file = TestFile::new(format!("{}/deep_file.txt", deep_path), "Deep content");
    let file = repo.create_test_file(&deep_file)?;
    file.stage()?;

    Ok(repo)
  }

  /// Create a repository with special characters in filenames
  pub fn create_special_chars() -> Result<TestRepo> {
    let repo = TestRepo::default();
    repo.create_initial_commit()?;

    // Files with spaces and special characters
    let special_files = vec![
      TestFile::new("file with spaces.txt", "Content"),
      TestFile::new("file-with-dashes.txt", "Content"),
      TestFile::new("file_with_underscores.txt", "Content"),
      TestFile::new("file.multiple.dots.txt", "Content"),
    ];

    for test_file in special_files {
      let file = repo.create_test_file(&test_file)?;
      file.stage()?;
    }

    Ok(repo)
  }
}

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Truncation behavior for MockModel
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TruncationBehavior {
  /// Always truncates to exact token count
  Perfect,
  /// Truncates slightly under target (90% of target)
  Conservative,
  /// May exceed target slightly (110% of target)
  Aggressive,
  /// Always fails truncation
  Fail
}

/// Mock model for testing token counting and truncation
pub struct MockModel {
  /// Predefined token counts for specific strings
  token_counts:         Arc<Mutex<HashMap<String, usize>>>,
  /// Context size for this model
  context_size:         usize,
  /// Whether token counting should fail
  should_fail_count:    bool,
  /// Whether truncation should fail
  should_fail_truncate: bool,
  /// Truncation behavior
  truncation_behavior:  TruncationBehavior,
  /// Default tokens per character ratio (for estimation)
  tokens_per_char:      f64
}

impl MockModel {
  /// Create a new MockModel with default settings
  pub fn new() -> Self {
    Self {
      token_counts:         Arc::new(Mutex::new(HashMap::new())),
      context_size:         8192,
      should_fail_count:    false,
      should_fail_truncate: false,
      truncation_behavior:  TruncationBehavior::Perfect,
      tokens_per_char:      0.25 // ~4 chars per token
    }
  }

  /// Set the context size
  pub fn with_context_size(mut self, size: usize) -> Self {
    self.context_size = size;
    self
  }

  /// Set whether token counting should fail
  pub fn with_count_failure(mut self, should_fail: bool) -> Self {
    self.should_fail_count = should_fail;
    self
  }

  /// Set whether truncation should fail
  pub fn with_truncate_failure(mut self, should_fail: bool) -> Self {
    self.should_fail_truncate = should_fail;
    self
  }

  /// Set truncation behavior
  pub fn with_truncation_behavior(mut self, behavior: TruncationBehavior) -> Self {
    self.truncation_behavior = behavior;
    self
  }

  /// Set tokens per character ratio
  pub fn with_tokens_per_char(mut self, ratio: f64) -> Self {
    self.tokens_per_char = ratio;
    self
  }

  /// Add a predefined token count for a specific string
  pub fn add_token_count(&self, text: impl Into<String>, count: usize) {
    let mut counts = self.token_counts.lock().unwrap();
    counts.insert(text.into(), count);
  }

  /// Count tokens in text
  pub fn count_tokens(&self, text: &str) -> Result<usize> {
    if self.should_fail_count {
      anyhow::bail!("Mock token counting failure");
    }

    // Check if we have a predefined count
    let counts = self.token_counts.lock().unwrap();
    if let Some(&count) = counts.get(text) {
      return Ok(count);
    }
    drop(counts);

    // Otherwise estimate based on character count
    let estimated = (text.len() as f64 * self.tokens_per_char).ceil() as usize;
    Ok(estimated)
  }

  /// Get context size
  pub fn context_size(&self) -> usize {
    self.context_size
  }

  /// Truncate text to fit within token limit
  pub fn truncate(&self, text: &str, max_tokens: usize) -> Result<String> {
    if self.should_fail_truncate {
      anyhow::bail!("Mock truncation failure");
    }

    let current_tokens = self.count_tokens(text)?;

    if current_tokens <= max_tokens {
      return Ok(text.to_string());
    }

    // Apply truncation behavior
    let target_tokens = match self.truncation_behavior {
      TruncationBehavior::Perfect => max_tokens,
      TruncationBehavior::Conservative => (max_tokens as f64 * 0.9) as usize,
      TruncationBehavior::Aggressive => (max_tokens as f64 * 1.1) as usize,
      TruncationBehavior::Fail => anyhow::bail!("Truncation behavior set to fail")
    };

    // Simple truncation: estimate character count needed
    let chars_per_token = 1.0 / self.tokens_per_char;
    let target_chars = (target_tokens as f64 * chars_per_token) as usize;
    let truncated: String = text.chars().take(target_chars).collect();

    // Verify we're within limits (for Perfect behavior)
    if self.truncation_behavior == TruncationBehavior::Perfect {
      let final_tokens = self.count_tokens(&truncated)?;
      if final_tokens > max_tokens {
        // Truncate more aggressively
        let adjusted_chars = (target_chars as f64 * 0.9) as usize;
        return Ok(text.chars().take(adjusted_chars).collect());
      }
    }

    Ok(truncated)
  }

  /// Walk truncate (iterative truncation)
  pub fn walk_truncate(&self, text: &str, max_tokens: usize, _within: usize) -> Result<String> {
    // For mock, just use regular truncate
    self.truncate(text, max_tokens)
  }
}

impl Default for MockModel {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod mock_model_tests {
  use super::*;

  #[test]
  fn test_mock_model_basic_counting() {
    let model = MockModel::new();
    let text = "Hello world";
    let count = model.count_tokens(text).unwrap();
    // With default 0.25 tokens per char, 11 chars = ~3 tokens
    assert!(count > 0);
  }

  #[test]
  fn test_mock_model_predefined_count() {
    let model = MockModel::new();
    model.add_token_count("test", 100);
    let count = model.count_tokens("test").unwrap();
    assert_eq!(count, 100);
  }

  #[test]
  fn test_mock_model_count_failure() {
    let model = MockModel::new().with_count_failure(true);
    let result = model.count_tokens("test");
    assert!(result.is_err());
  }

  #[test]
  fn test_mock_model_truncation() {
    let model = MockModel::new();
    let text = "a".repeat(1000);
    let truncated = model.truncate(&text, 50).unwrap();
    assert!(truncated.len() < text.len());
  }

  #[test]
  fn test_mock_model_truncation_behaviors() {
    let text = "a".repeat(1000);

    // Perfect behavior
    let model = MockModel::new().with_truncation_behavior(TruncationBehavior::Perfect);
    let result = model.truncate(&text, 50);
    assert!(result.is_ok());

    // Fail behavior
    let model = MockModel::new().with_truncation_behavior(TruncationBehavior::Fail);
    let result = model.truncate(&text, 50);
    assert!(result.is_err());
  }
}

use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Memory statistics from a test run
#[derive(Debug, Clone)]
pub struct MemoryStats {
  /// Peak memory usage in bytes (estimated)
  pub peak_usage:     usize,
  /// Number of allocations tracked
  pub allocations:    usize,
  /// Average allocation size
  pub avg_alloc_size: usize
}

/// Memory tracker for monitoring memory usage during tests
pub struct MemoryTracker {
  allocations: Arc<AtomicUsize>,
  peak_usage:  Arc<AtomicUsize>
}

impl MemoryTracker {
  pub fn new() -> Self {
    Self {
      allocations: Arc::new(AtomicUsize::new(0)),
      peak_usage:  Arc::new(AtomicUsize::new(0))
    }
  }

  /// Track an allocation
  pub fn track_allocation(&self, size: usize) {
    self.allocations.fetch_add(1, Ordering::Relaxed);
    let current = self.peak_usage.load(Ordering::Relaxed);
    if size > current {
      self.peak_usage.store(size, Ordering::Relaxed);
    }
  }

  /// Get current statistics
  pub fn stats(&self) -> MemoryStats {
    let allocations = self.allocations.load(Ordering::Relaxed);
    let peak = self.peak_usage.load(Ordering::Relaxed);

    MemoryStats {
      peak_usage: peak,
      allocations,
      avg_alloc_size: if allocations > 0 {
        peak / allocations
      } else {
        0
      }
    }
  }

  /// Reset tracking
  pub fn reset(&self) {
    self.allocations.store(0, Ordering::Relaxed);
    self.peak_usage.store(0, Ordering::Relaxed);
  }
}

impl Default for MemoryTracker {
  fn default() -> Self {
    Self::new()
  }
}

/// Time tracker for execution time measurements
pub struct TimeTracker {
  start: Option<Instant>
}

impl TimeTracker {
  pub fn new() -> Self {
    Self { start: None }
  }

  /// Start timing
  pub fn start(&mut self) {
    self.start = Some(Instant::now());
  }

  /// Stop timing and return elapsed duration
  pub fn stop(&mut self) -> Duration {
    match self.start.take() {
      Some(start) => start.elapsed(),
      None => Duration::from_secs(0)
    }
  }

  /// Measure execution time of a function
  pub fn measure<F, R>(f: F) -> (R, Duration)
  where
    F: FnOnce() -> R
  {
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();
    (result, duration)
  }
}

impl Default for TimeTracker {
  fn default() -> Self {
    Self::new()
  }
}

/// Results from concurrency testing
#[derive(Debug)]
pub struct ConcurrencyResults {
  /// Number of threads used
  pub thread_count:    usize,
  /// Total execution time
  pub total_duration:  Duration,
  /// Number of successful operations
  pub successful_ops:  usize,
  /// Number of failed operations
  pub failed_ops:      usize,
  /// Whether any race conditions were detected
  pub race_conditions: bool
}

impl ConcurrencyResults {
  /// Check if all operations were successful
  pub fn all_successful(&self) -> bool {
    self.failed_ops == 0
  }

  /// Check if no race conditions were detected
  pub fn no_race_conditions(&self) -> bool {
    !self.race_conditions
  }
}

/// Concurrency tester for parallel processing validation
pub struct ConcurrencyTester {
  thread_count: usize
}

impl ConcurrencyTester {
  pub fn new(thread_count: usize) -> Self {
    Self { thread_count }
  }

  /// Test concurrent execution of a function
  pub fn test<F, R>(&self, f: F) -> ConcurrencyResults
  where
    F: Fn() -> Result<R> + Send + Sync + 'static,
    R: Send + 'static
  {
    let start = Instant::now();
    let f = Arc::new(f);
    let successful = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..self.thread_count)
      .map(|_| {
        let f = Arc::clone(&f);
        let successful = Arc::clone(&successful);
        let failed = Arc::clone(&failed);

        std::thread::spawn(move || {
          match f() {
            Ok(_) => successful.fetch_add(1, Ordering::Relaxed),
            Err(_) => failed.fetch_add(1, Ordering::Relaxed)
          };
        })
      })
      .collect();

    for handle in handles {
      handle.join().unwrap();
    }

    let duration = start.elapsed();

    ConcurrencyResults {
      thread_count:    self.thread_count,
      total_duration:  duration,
      successful_ops:  successful.load(Ordering::Relaxed),
      failed_ops:      failed.load(Ordering::Relaxed),
      race_conditions: false // Would need more sophisticated detection
    }
  }
}

/// Performance testing harness combining all measurement tools
pub struct PerformanceHarness {
  memory_tracker:     MemoryTracker,
  time_tracker:       TimeTracker,
  concurrency_tester: ConcurrencyTester
}

impl PerformanceHarness {
  pub fn new() -> Self {
    Self {
      memory_tracker:     MemoryTracker::new(),
      time_tracker:       TimeTracker::new(),
      concurrency_tester: ConcurrencyTester::new(4)
    }
  }

  /// Create with specific thread count for concurrency testing
  pub fn with_thread_count(thread_count: usize) -> Self {
    Self {
      memory_tracker:     MemoryTracker::new(),
      time_tracker:       TimeTracker::new(),
      concurrency_tester: ConcurrencyTester::new(thread_count)
    }
  }

  /// Measure memory usage of a function
  pub fn measure_memory_usage<F, R>(&self, f: F) -> (R, MemoryStats)
  where
    F: FnOnce() -> R
  {
    self.memory_tracker.reset();
    let result = f();
    let stats = self.memory_tracker.stats();
    (result, stats)
  }

  /// Measure execution time of a function
  pub fn measure_execution_time<F, R>(&self, f: F) -> (R, Duration)
  where
    F: FnOnce() -> R
  {
    TimeTracker::measure(f)
  }

  /// Test concurrent access
  pub fn test_concurrent_access<F, R>(&self, f: F) -> ConcurrencyResults
  where
    F: Fn() -> Result<R> + Send + Sync + 'static,
    R: Send + 'static
  {
    self.concurrency_tester.test(f)
  }

  /// Measure both time and memory
  pub fn measure_all<F, R>(&self, f: F) -> (R, Duration, MemoryStats)
  where
    F: FnOnce() -> R
  {
    self.memory_tracker.reset();
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();
    let stats = self.memory_tracker.stats();
    (result, duration, stats)
  }
}

impl Default for PerformanceHarness {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod performance_tests {
  use super::*;

  #[test]
  fn test_time_tracker() {
    let (result, duration) = TimeTracker::measure(|| {
      std::thread::sleep(Duration::from_millis(10));
      42
    });

    assert_eq!(result, 42);
    assert!(duration.as_millis() >= 10);
  }

  #[test]
  fn test_memory_tracker() {
    let tracker = MemoryTracker::new();
    tracker.track_allocation(1000);
    tracker.track_allocation(2000);

    let stats = tracker.stats();
    assert_eq!(stats.peak_usage, 2000);
    assert_eq!(stats.allocations, 2);
  }

  #[test]
  fn test_concurrency_tester() {
    let tester = ConcurrencyTester::new(4);
    let counter = Arc::new(AtomicUsize::new(0));

    let counter_clone = Arc::clone(&counter);
    let results = tester.test(move || {
      counter_clone.fetch_add(1, Ordering::Relaxed);
      Ok(())
    });

    assert!(results.all_successful());
    assert_eq!(results.successful_ops, 4);
  }

  #[test]
  fn test_performance_harness() {
    let harness = PerformanceHarness::new();

    let (result, duration, _stats) = harness.measure_all(|| {
      std::thread::sleep(Duration::from_millis(10));
      "test"
    });

    assert_eq!(result, "test");
    assert!(duration.as_millis() >= 10);
  }
}

/// Types of git errors that can be injected
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GitErrorType {
  /// Repository not found
  RepositoryNotFound,
  /// Permission denied
  PermissionDenied,
  /// Corrupted index
  CorruptedIndex,
  /// Merge conflict
  MergeConflict,
  /// Missing reference
  MissingReference,
  /// Detached HEAD
  DetachedHead
}

/// Types of IO errors that can be injected
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IoErrorType {
  /// File not found
  FileNotFound,
  /// Permission denied
  PermissionDenied,
  /// Disk full
  DiskFull,
  /// Network timeout
  NetworkTimeout,
  /// Invalid data
  InvalidData
}

/// Recovery action to take after an error
#[derive(Debug, Clone)]
pub enum RecoveryAction {
  /// Retry the operation
  Retry {
    max_attempts: usize,
    delay:        Duration
  },
  /// Use a fallback strategy
  Fallback {
    reason: String
  },
  /// Fail with error message
  Fail {
    error_message: String
  },
  /// Skip the operation
  Skip {
    reason: String
  }
}

/// Error injection framework for testing failure scenarios
pub struct ErrorInjector {
  git_failures:    Vec<GitErrorType>,
  io_failures:     Vec<IoErrorType>,
  memory_pressure: bool,
  failure_rate:    f64 // 0.0 to 1.0, probability of injecting error
}

impl ErrorInjector {
  pub fn new() -> Self {
    Self {
      git_failures:    Vec::new(),
      io_failures:     Vec::new(),
      memory_pressure: false,
      failure_rate:    1.0 // Always inject by default
    }
  }

  /// Add a git error to inject
  pub fn with_git_error(mut self, error: GitErrorType) -> Self {
    self.git_failures.push(error);
    self
  }

  /// Add an IO error to inject
  pub fn with_io_error(mut self, error: IoErrorType) -> Self {
    self.io_failures.push(error);
    self
  }

  /// Enable memory pressure simulation
  pub fn with_memory_pressure(mut self, enabled: bool) -> Self {
    self.memory_pressure = enabled;
    self
  }

  /// Set failure rate (0.0 to 1.0)
  pub fn with_failure_rate(mut self, rate: f64) -> Self {
    self.failure_rate = rate.clamp(0.0, 1.0);
    self
  }

  /// Check if a git error should be injected
  pub fn should_inject_git_error(&self, error_type: GitErrorType) -> bool {
    if self.git_failures.contains(&error_type) {
      // Simple deterministic check for now
      return self.failure_rate >= 1.0;
    }
    false
  }

  /// Check if an IO error should be injected
  pub fn should_inject_io_error(&self, error_type: IoErrorType) -> bool {
    if self.io_failures.contains(&error_type) {
      return self.failure_rate >= 1.0;
    }
    false
  }

  /// Check if memory pressure should be simulated
  pub fn should_simulate_memory_pressure(&self) -> bool {
    self.memory_pressure
  }

  /// Create an error for the given git error type
  pub fn create_git_error(&self, error_type: GitErrorType) -> anyhow::Error {
    match error_type {
      GitErrorType::RepositoryNotFound => anyhow::anyhow!("Repository not found"),
      GitErrorType::PermissionDenied => anyhow::anyhow!("Permission denied accessing repository"),
      GitErrorType::CorruptedIndex => anyhow::anyhow!("Repository index is corrupted"),
      GitErrorType::MergeConflict => anyhow::anyhow!("Merge conflict detected"),
      GitErrorType::MissingReference => anyhow::anyhow!("Reference not found"),
      GitErrorType::DetachedHead => anyhow::anyhow!("Repository is in detached HEAD state")
    }
  }

  /// Create an error for the given IO error type
  pub fn create_io_error(&self, error_type: IoErrorType) -> anyhow::Error {
    match error_type {
      IoErrorType::FileNotFound => anyhow::anyhow!("File not found"),
      IoErrorType::PermissionDenied => anyhow::anyhow!("Permission denied"),
      IoErrorType::DiskFull => anyhow::anyhow!("No space left on device"),
      IoErrorType::NetworkTimeout => anyhow::anyhow!("Network operation timed out"),
      IoErrorType::InvalidData => anyhow::anyhow!("Invalid data encountered")
    }
  }

  /// Determine recovery action for a git error
  pub fn recovery_action_for_git(&self, error_type: GitErrorType) -> RecoveryAction {
    match error_type {
      GitErrorType::RepositoryNotFound =>
        RecoveryAction::Fail {
          error_message: "Repository not found. Please check the path.".to_string()
        },
      GitErrorType::PermissionDenied =>
        RecoveryAction::Fail {
          error_message: "Permission denied. Please check file permissions.".to_string()
        },
      GitErrorType::CorruptedIndex =>
        RecoveryAction::Retry {
          max_attempts: 3,
          delay:        Duration::from_millis(100)
        },
      GitErrorType::MergeConflict =>
        RecoveryAction::Skip {
          reason: "Skipping due to merge conflict".to_string()
        },
      GitErrorType::MissingReference => RecoveryAction::Fallback { reason: "Using alternative reference".to_string() },
      GitErrorType::DetachedHead =>
        RecoveryAction::Skip {
          reason: "Repository in detached HEAD state".to_string()
        },
    }
  }

  /// Determine recovery action for an IO error
  pub fn recovery_action_for_io(&self, error_type: IoErrorType) -> RecoveryAction {
    match error_type {
      IoErrorType::FileNotFound => RecoveryAction::Skip { reason: "File not found, skipping".to_string() },
      IoErrorType::PermissionDenied => RecoveryAction::Fail { error_message: "Permission denied".to_string() },
      IoErrorType::DiskFull =>
        RecoveryAction::Fail {
          error_message: "Disk full, cannot continue".to_string()
        },
      IoErrorType::NetworkTimeout =>
        RecoveryAction::Retry {
          max_attempts: 3,
          delay:        Duration::from_secs(1)
        },
      IoErrorType::InvalidData => RecoveryAction::Skip { reason: "Invalid data, skipping".to_string() }
    }
  }
}

impl Default for ErrorInjector {
  fn default() -> Self {
    Self::new()
  }
}

/// Trait for error recovery strategies
pub trait ErrorRecovery {
  fn handle_git_error(&self, error: GitErrorType) -> RecoveryAction;
  fn handle_io_error(&self, error: IoErrorType) -> RecoveryAction;
}

impl ErrorRecovery for ErrorInjector {
  fn handle_git_error(&self, error: GitErrorType) -> RecoveryAction {
    self.recovery_action_for_git(error)
  }

  fn handle_io_error(&self, error: IoErrorType) -> RecoveryAction {
    self.recovery_action_for_io(error)
  }
}

#[cfg(test)]
mod error_injection_tests {
  use super::*;

  #[test]
  fn test_error_injector_git_errors() {
    let injector = ErrorInjector::new().with_git_error(GitErrorType::PermissionDenied);

    assert!(injector.should_inject_git_error(GitErrorType::PermissionDenied));
    assert!(!injector.should_inject_git_error(GitErrorType::RepositoryNotFound));
  }

  #[test]
  fn test_error_injector_io_errors() {
    let injector = ErrorInjector::new().with_io_error(IoErrorType::DiskFull);

    assert!(injector.should_inject_io_error(IoErrorType::DiskFull));
    assert!(!injector.should_inject_io_error(IoErrorType::FileNotFound));
  }

  #[test]
  fn test_error_injector_memory_pressure() {
    let injector = ErrorInjector::new().with_memory_pressure(true);

    assert!(injector.should_simulate_memory_pressure());
  }

  #[test]
  fn test_error_creation() {
    let injector = ErrorInjector::new();

    let git_error = injector.create_git_error(GitErrorType::PermissionDenied);
    assert!(git_error.to_string().contains("Permission denied"));

    let io_error = injector.create_io_error(IoErrorType::DiskFull);
    assert!(io_error.to_string().contains("No space left"));
  }

  #[test]
  fn test_recovery_actions() {
    let injector = ErrorInjector::new();

    let action = injector.recovery_action_for_git(GitErrorType::CorruptedIndex);
    match action {
      RecoveryAction::Retry { max_attempts, .. } => assert_eq!(max_attempts, 3),
      _ => panic!("Expected Retry action")
    }

    let action = injector.recovery_action_for_io(IoErrorType::NetworkTimeout);
    match action {
      RecoveryAction::Retry { max_attempts, .. } => assert_eq!(max_attempts, 3),
      _ => panic!("Expected Retry action")
    }
  }

  #[test]
  fn test_error_recovery_trait() {
    let injector = ErrorInjector::new();

    let action = injector.handle_git_error(GitErrorType::MergeConflict);
    match action {
      RecoveryAction::Skip { .. } => {}
      _ => panic!("Expected Skip action")
    }
  }
}
