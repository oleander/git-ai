//! Diff processing and parsing utilities.
//!
//! This module handles parsing git diffs into structured data
//! and provides utilities for working with diff content.

pub mod parser;
pub mod traits;

pub use parser::{ParsedFile, parse_diff};
pub use traits::{FilePath, Utf8String, DiffDeltaPath};