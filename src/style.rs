use std::env;
use std::path::PathBuf;

use console::style;

pub trait Styled {
  fn relative_path(&self) -> String;
}

impl Styled for PathBuf {
  fn relative_path(&self) -> String {
    let current_dir = env::current_dir().unwrap();
    let relative_path = self.strip_prefix(&current_dir).unwrap();
    style(relative_path.display()).italic().to_string()
  }
}
