//! Error handling utilities for the git-ai CLI tool.
//!
//! This module provides helpers for detecting and handling specific error types,
//! particularly authentication failures from the OpenAI API.

use anyhow::Error;

/// Checks if an error represents an OpenAI API authentication failure.
///
/// This function detects various authentication failure patterns including:
/// - OpenAI-specific API key errors (invalid_api_key, incorrect API key)
/// - Generic authentication/authorization failures
/// - HTTP-level errors that typically indicate authentication issues when calling OpenAI
///
/// # Arguments
///
/// * `error` - The error to check
///
/// # Returns
///
/// `true` if the error appears to be an authentication failure, `false` otherwise
///
/// # Examples
///
/// ```
/// use anyhow::anyhow;
/// use ai::error::is_openai_auth_error;
///
/// let error = anyhow!("invalid_api_key: Incorrect API key provided");
/// assert!(is_openai_auth_error(&error));
/// ```
pub fn is_openai_auth_error(error: &Error) -> bool {
  let msg = error.to_string().to_lowercase();

  // OpenAI-specific API key errors
  msg.contains("invalid_api_key") ||
  msg.contains("incorrect api key") ||
  msg.contains("openai api authentication failed") ||

  // Generic auth failures (scoped to avoid false positives)
  (msg.contains("authentication") && msg.contains("openai")) ||
  (msg.contains("unauthorized") && msg.contains("openai")) ||

  // HTTP errors that typically indicate auth issues with OpenAI
  // This pattern catches connection issues when the API key is malformed
  (msg.contains("http error") && msg.contains("error sending request"))
}

#[cfg(test)]
mod tests {
  use super::*;
  use anyhow::anyhow;

  #[test]
  fn test_detects_invalid_api_key() {
    let error = anyhow!("invalid_api_key: Incorrect API key provided");
    assert!(is_openai_auth_error(&error));
  }

  #[test]
  fn test_detects_incorrect_api_key() {
    let error = anyhow!("Incorrect API key provided: sk-xxxxx");
    assert!(is_openai_auth_error(&error));
  }

  #[test]
  fn test_detects_openai_auth_failed() {
    let error = anyhow!("OpenAI API authentication failed: http error");
    assert!(is_openai_auth_error(&error));
  }

  #[test]
  fn test_detects_http_error_sending_request() {
    let error = anyhow!("http error: error sending request");
    assert!(is_openai_auth_error(&error));
  }

  #[test]
  fn test_detects_openai_specific_auth() {
    let error = anyhow!("OpenAI authentication failed");
    assert!(is_openai_auth_error(&error));
  }

  #[test]
  fn test_ignores_generic_auth_errors() {
    // Should not match generic auth errors without OpenAI context
    let error = anyhow!("Database authentication timeout");
    assert!(!is_openai_auth_error(&error));

    let error = anyhow!("OAuth2 unauthorized redirect");
    assert!(!is_openai_auth_error(&error));
  }

  #[test]
  fn test_ignores_unrelated_errors() {
    let error = anyhow!("File not found");
    assert!(!is_openai_auth_error(&error));
  }
}
