use std::time::Instant;

use tracing::debug;

pub struct Profile {
  name:  String,
  start: Instant
}

impl Profile {
  pub fn new(name: &str) -> Self {
    Self { name: name.to_string(), start: Instant::now() }
  }
}

impl Drop for Profile {
  fn drop(&mut self) {
    let elapsed = self.start.elapsed();
    debug!("{} took {:?}", self.name, elapsed);
  }
}

pub fn span(name: &str) -> Profile {
  debug!("Starting {}", name);
  Profile::new(name)
}
