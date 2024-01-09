use std::env;
use std::path::PathBuf;
pub trait Styled {
  fn relative_path(&self) -> PathBuf;
}

impl Styled for PathBuf {
  fn relative_path(&self) -> PathBuf {
    let current_dir = env::current_dir().unwrap();
    let relative_path = self.strip_prefix(&current_dir).unwrap_or(current_dir.as_path());
    relative_path.to_path_buf()
  }
}
