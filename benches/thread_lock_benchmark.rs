use std::time::Instant;

use git2::Repository;
use ai::hook::PatchRepository;
use ai::model::Model;

fn main() -> anyhow::Result<()> {
  println!("Git-AI Thread Lock Performance Benchmark\n");

  // Open the repository
  let repo = Repository::open_from_env()?;
  let model = Model::from("gpt-4o-mini");
  let max_tokens = 4096;

  // Get the current HEAD tree
  let tree = repo.head().ok().and_then(|head| head.peel_to_tree().ok());

  // Warm up run
  println!("Warming up...");
  let _ = repo.to_patch(tree.clone(), max_tokens, model.clone())?;

  // Benchmark runs
  let num_runs = 5;
  let mut total_duration = std::time::Duration::ZERO;

  println!("\nRunning {} benchmark iterations:", num_runs);
  for i in 1..=num_runs {
    let start = Instant::now();
    let patch = repo.to_patch(tree.clone(), max_tokens, model.clone())?;
    let duration = start.elapsed();

    println!("  Run {}: {:?} (patch size: {} bytes)", i, duration, patch.len());
    total_duration += duration;
  }

  let avg_duration = total_duration / num_runs as u32;
  println!("\nAverage execution time: {:?}", avg_duration);

  // Performance summary
  println!("\nOptimizations applied:");
  println!("  ✓ Replaced RwLock with lock-free channels");
  println!("  ✓ Pre-allocated tokens to reduce atomic contention");
  println!("  ✓ Global thread pool to avoid creation overhead");
  println!("  ✓ Async file I/O operations");
  println!("  ✓ Optimized chunk processing with local token counters");

  Ok(())
}
