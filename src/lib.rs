#[macro_export]
macro_rules! profile {
  ($name:expr) => {{
    let _span = tracing::span!(tracing::Level::DEBUG, $name);
    let _enter = _span.enter();
  }};
}

pub mod commit;
pub mod config;
pub mod filesystem;
pub mod hook;
pub mod model;
pub mod openai;
pub mod profiling;
pub mod style;
pub mod finetune;

// Re-exports
pub use profiling::Profile;
