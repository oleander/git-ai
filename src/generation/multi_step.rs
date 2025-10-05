//! Multi-step commit message generation.
//!
//! Implements a sophisticated analysis pipeline:
//! 1. Parse diff into individual files
//! 2. Analyze each file (lines changed, category, impact)
//! 3. Score files by impact
//! 4. Generate message candidates
//! 5. Select best candidate

use anyhow::Result;
use async_openai::config::OpenAIConfig;
use async_openai::Client;

pub mod analysis;
pub mod scoring;
pub mod candidates;
pub mod local;

// Re-export commonly used types and functions
pub use analysis::{FileAnalysis, analyze_file, analyze_file_via_api, ParsedFile};
pub use scoring::{calculate_impact_scores, ImpactScore};
pub use candidates::{generate_candidates, select_best_candidate};

/// Main entry point for multi-step generation with API
pub async fn generate_with_api(
    client: &Client<OpenAIConfig>,
    model: &str,
    diff: &str,
    max_length: Option<usize>,
) -> Result<String> {
    // This will be moved from multi_step_integration.rs generate_commit_message_multi_step
    crate::multi_step_integration::generate_commit_message_multi_step(client, model, diff, max_length).await
}

/// Main entry point for local multi-step generation (no API)
pub fn generate_local(
    diff: &str,
    max_length: Option<usize>,
) -> Result<String> {
    // This will be moved from multi_step_integration.rs generate_commit_message_local
    crate::multi_step_integration::generate_commit_message_local(diff, max_length)
}