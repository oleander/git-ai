//! Diff processing and parsing utilities.
//!
//! This module handles parsing git diffs into structured data
//! and provides utilities for working with diff content.

pub mod parser;
pub mod traits;

pub use parser::{parse_diff, ParsedFile};
pub use traits::{DiffDeltaPath, FilePath, Utf8String};
