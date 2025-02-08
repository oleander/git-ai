use std::time::{Duration, Instant};

use colored::Colorize;

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
    if log::log_enabled!(log::Level::Debug) {
      let duration = self.elapsed();
      eprintln!("{}: {:.2?}", self.name.blue(), duration);
    }
  }
}

#[macro_export]
macro_rules! profile {
  ($name:expr) => {
    // Currently a no-op, but can be expanded for actual profiling
    let _profile_span = $name;
  };
}
