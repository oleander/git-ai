pub mod commit;
pub mod config;
pub mod hook;
pub mod style;
pub mod model;
pub mod filesystem;
pub mod openai;
pub mod profiling;
pub mod function_calling;
pub mod multi_step_analysis;
pub mod multi_step_integration;
pub mod simple_multi_step;
pub mod debug_output;
pub mod generation;

// Re-exports
pub use profiling::Profile;
