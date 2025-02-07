use std::time::Duration;

mod common;

#[test]
fn test_profiling_basic() {
  let profile = ai::Profile::new("test_operation");
  std::thread::sleep(Duration::from_millis(10));
  let elapsed = profile.elapsed();
  assert!(elapsed >= Duration::from_millis(10));
}

#[test]
fn test_profiling_drop() {
  let _profile = ai::Profile::new("test_drop");
  // The profile will be dropped at the end of this scope
  // and should print the elapsed time to stderr
  std::thread::sleep(Duration::from_millis(10));
}

#[test]
fn test_profiling_multiple() {
  let profile1 = ai::Profile::new("operation1");
  std::thread::sleep(Duration::from_millis(10));
  let elapsed1 = profile1.elapsed();

  let profile2 = ai::Profile::new("operation2");
  std::thread::sleep(Duration::from_millis(20));
  let elapsed2 = profile2.elapsed();

  assert!(elapsed1 >= Duration::from_millis(10));
  assert!(elapsed2 >= Duration::from_millis(20));
}

#[test]
fn test_profiling_nested() {
  let outer = ai::Profile::new("outer");
  std::thread::sleep(Duration::from_millis(10));

  {
    let inner = ai::Profile::new("inner");
    std::thread::sleep(Duration::from_millis(10));
    assert!(inner.elapsed() >= Duration::from_millis(10));
  }

  assert!(outer.elapsed() >= Duration::from_millis(20));
}
