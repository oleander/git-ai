//! Local generation fallback (no API required).

use anyhow::Result;

pub fn generate_simple(diff: &str, max_length: usize) -> Result<String> {
  // This will be moved from simple_multi_step.rs generate_commit_message_simple_local
  // For now, use the local multi-step approach
  crate::multi_step_integration::generate_commit_message_local(diff, Some(max_length))
}
