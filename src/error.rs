//! Error handling utilities for the git-ai CLI tool.
//!
//! This module provides helpers for detecting and handling specific error types,
//! particularly authentication failures from the OpenAI API.
//!
//! # OpenAI Error Structure
//!
//! According to the official async-openai documentation:
//! - `OpenAIError::ApiError(ApiError)` contains structured error information from OpenAI
//! - `ApiError` has fields: `message`, `type`, `param`, and `code`
//! - Authentication errors have `code` set to `"invalid_api_key"`
//! - `OpenAIError::Reqwest(Error)` contains HTTP-level errors (connection issues, etc.)
//!
//! Reference: https://docs.rs/async-openai/latest/async_openai/error/

use anyhow::Error;
use async_openai::error::OpenAIError;

/// Checks if an error represents an OpenAI API authentication failure.
///
/// This function detects authentication failures by checking for:
/// 1. **Structured API errors** (preferred): Checks if the error contains an `OpenAIError::ApiError`
///    with `code` field set to `"invalid_api_key"` - this is the official OpenAI error code
///    for authentication failures.
/// 2. **String-based fallback**: As a fallback, checks for authentication-related keywords in
///    the error message for cases where the error has been wrapped or converted to a string.
///
/// This approach is based on the official OpenAI API error codes documentation and the
/// async-openai Rust library structure.
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
  // First, try to downcast to OpenAIError for accurate detection
  if let Some(openai_err) = error.downcast_ref::<OpenAIError>() {
    match openai_err {
      // Official OpenAI API error with structured error code
      OpenAIError::ApiError(api_err) => {
        // Check for the official invalid_api_key error code
        if api_err.code.as_deref() == Some("invalid_api_key") {
          return true;
        }
        // Also check for authentication-related types
        if let Some(err_type) = &api_err.r#type {
          if err_type.contains("authentication") || err_type.contains("invalid_request_error") {
            // For invalid_request_error, check if the message mentions API key
            if err_type == "invalid_request_error" && api_err.message.to_lowercase().contains("api key") {
              return true;
            }
          }
        }
      }
      // HTTP-level errors (connection failures, malformed requests, etc.)
      OpenAIError::Reqwest(_) => {
        // Reqwest errors for auth issues typically manifest as connection errors
        // when the API key format is completely invalid (e.g., "dl://BA7...")
        let msg = error.to_string().to_lowercase();
        if msg.contains("error sending request") || msg.contains("connection") {
          return true;
        }
      }
      _ => {}
    }
  }

  // Fallback: String-based detection for wrapped errors
  let msg = error.to_string().to_lowercase();

  // OpenAI-specific API key errors (from API responses)
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
  use anyhow::anyhow;
  use async_openai::error::{ApiError, OpenAIError};

  use super::*;

  // Tests for structured OpenAIError detection (preferred method)

  #[test]
  fn test_detects_structured_invalid_api_key() {
    let api_error = ApiError {
      message: "Incorrect API key provided: dl://BA7...".to_string(),
      r#type:  Some("invalid_request_error".to_string()),
      param:   None,
      code:    Some("invalid_api_key".to_string())
    };
    let openai_error = OpenAIError::ApiError(api_error);
    let error: anyhow::Error = openai_error.into();
    assert!(is_openai_auth_error(&error));
  }

  #[test]
  fn test_detects_invalid_request_with_api_key_message() {
    let api_error = ApiError {
      message: "You must provide a valid API key".to_string(),
      r#type:  Some("invalid_request_error".to_string()),
      param:   None,
      code:    None
    };
    let openai_error = OpenAIError::ApiError(api_error);
    let error: anyhow::Error = openai_error.into();
    assert!(is_openai_auth_error(&error));
  }

  #[test]
  fn test_detects_reqwest_error_sending_request() {
    // Simulate a wrapped reqwest error by using anyhow
    // In production, malformed API keys cause "error sending request" from reqwest
    let error = anyhow!("http error: error sending request");
    assert!(is_openai_auth_error(&error));
  }

  #[test]
  fn test_ignores_structured_non_auth_error() {
    let api_error = ApiError {
      message: "Model not found".to_string(),
      r#type:  Some("invalid_request_error".to_string()),
      param:   Some("model".to_string()),
      code:    Some("model_not_found".to_string())
    };
    let openai_error = OpenAIError::ApiError(api_error);
    let error: anyhow::Error = openai_error.into();
    assert!(!is_openai_auth_error(&error));
  }

  // Tests for string-based fallback detection (for wrapped errors)

  #[test]
  fn test_detects_invalid_api_key_string() {
    let error = anyhow!("invalid_api_key: Incorrect API key provided");
    assert!(is_openai_auth_error(&error));
  }

  #[test]
  fn test_detects_incorrect_api_key_string() {
    let error = anyhow!("Incorrect API key provided: sk-xxxxx");
    assert!(is_openai_auth_error(&error));
  }

  #[test]
  fn test_detects_openai_auth_failed_string() {
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

  #[test]
  fn test_ignores_non_auth_openai_errors() {
    let error = anyhow!("OpenAI rate limit exceeded");
    assert!(!is_openai_auth_error(&error));
  }
}
