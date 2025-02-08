pub mod commit;
pub mod config;
pub mod hook;
pub mod style;
pub mod model;
pub mod filesystem;
pub mod openai;
pub mod ollama;
pub mod client;
pub mod profiling;

// Re-exports
pub use client::{call, is_model_available, Request, Response};
pub use profiling::Profile;
