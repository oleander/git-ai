pub mod commit;
pub mod config;
pub mod hook;
pub mod style;
pub mod model;
pub mod filesystem;
pub mod openai;
pub mod ollama;
<<<<<<< HEAD
=======
pub mod client;

// Re-export the client module as the main interface
pub use client::{call, is_model_available, Request, Response};
>>>>>>> fbe8ab1 (<think>)
