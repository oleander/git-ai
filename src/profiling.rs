use std::time::{Duration, Instant};

use colored::Colorize;

use crate::debug_output;

pub struct Profile {
  start: Instant,
  name:  String
}

impl Profile {
  pub fn new(name: impl Into<String>) -> Self {
    Self { start: Instant::now(), name: name.into() }
  }

  pub fn elapsed(&self) -> Duration {
    self.start.elapsed()
  }
}

impl Drop for Profile {
  fn drop(&mut self) {
    let duration = self.elapsed();

    // Record timing in debug session if available
    debug_output::record_timing(&self.name, duration);

    // Always show profiling in debug builds, otherwise respect log level
    #[cfg(debug_assertions)]
    {
      eprintln!("{}: {:.2?}", self.name.blue(), duration);
    }

    #[cfg(not(debug_assertions))]
    if log::log_enabled!(log::Level::Debug) {
      eprintln!("{}: {:.2?}", self.name.blue(), duration);
    }
  }
}

#[macro_export]
macro_rules! profile {
  ($name:expr) => {
    let _profile = $crate::Profile::new($name);
  };
}

/// Helper function to profile a block of code and return its result
pub fn profile_fn<F, T>(name: &str, f: F) -> T
where
  F: FnOnce() -> T
{
  let profile = Profile::new(name);
  let result = f();
  drop(profile);
  result
}
