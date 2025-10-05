//! Fallback strategy orchestration for commit message generation.
//!
//! Implements a strategy pattern that tries multiple generation
//! approaches in order until one succeeds:
//! 1. Multi-step with OpenAI API
//! 2. Local multi-step analysis
//! 3. Single-step API call
//!
//! # Example
//! ```rust
//! use ai::generation::fallback::generate_with_fallback;
//! use ai::config::AppConfig;
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! let config = AppConfig::default();
//! let message = generate_with_fallback("test diff", &config).await?;
//! # Ok::<(), anyhow::Error>(())
//! # });
//! ```

use anyhow::{bail, Result};
use async_trait::async_trait;
use async_openai::config::OpenAIConfig;
use async_openai::Client;

use crate::config::AppConfig;
use crate::multi_step_integration::{generate_commit_message_multi_step, generate_commit_message_local};
use crate::{commit, model::Model};

/// Strategy for generating commit messages
#[async_trait]
pub trait GenerationStrategy: Send + Sync {
    /// Attempt to generate a commit message
    async fn generate(
        &self,
        diff: &str,
        config: &AppConfig,
    ) -> Result<String>;
    
    /// Name of this strategy (for logging)
    fn name(&self) -> &str;
    
    /// Whether this strategy requires an API key
    fn requires_api_key(&self) -> bool {
        false
    }
}

/// Multi-step generation using OpenAI API
pub struct MultiStepAPIStrategy {
    client: Client<OpenAIConfig>,
    model: String,
}

impl MultiStepAPIStrategy {
    pub fn new(config: &AppConfig) -> Result<Self> {
        let api_key = get_api_key(config)?;
        let openai_config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(openai_config);
        let model = config.model.clone().unwrap_or("gpt-4o-mini".to_string());
        
        Ok(Self { client, model })
    }
}

#[async_trait]
impl GenerationStrategy for MultiStepAPIStrategy {
    async fn generate(&self, diff: &str, config: &AppConfig) -> Result<String> {
        generate_commit_message_multi_step(&self.client, &self.model, diff, config.max_commit_length).await
    }
    
    fn name(&self) -> &str {
        "Multi-step API"
    }
    
    fn requires_api_key(&self) -> bool {
        true
    }
}

/// Local multi-step generation (no API)
pub struct LocalMultiStepStrategy;

#[async_trait]
impl GenerationStrategy for LocalMultiStepStrategy {
    async fn generate(&self, diff: &str, config: &AppConfig) -> Result<String> {
        // Note: generate_commit_message_local is synchronous
        let result = generate_commit_message_local(diff, config.max_commit_length)?;
        Ok(result)
    }
    
    fn name(&self) -> &str {
        "Local multi-step"
    }
}

/// Simple single-step API generation (original approach)
pub struct SingleStepAPIStrategy {
    model: Model,
}

impl SingleStepAPIStrategy {
    pub fn new(config: &AppConfig) -> Result<Self> {
        let _api_key = get_api_key(config)?; // Validate API key exists
        let model = config.model.as_ref()
            .map(|m| m.parse().unwrap_or_default())
            .unwrap_or_default();
        
        Ok(Self { model })
    }
}

#[async_trait]
impl GenerationStrategy for SingleStepAPIStrategy {
    async fn generate(&self, diff: &str, config: &AppConfig) -> Result<String> {
        // Use original single-step generation via commit::generate
        let max_tokens = config.max_tokens.unwrap_or(512);
        let response = commit::generate(diff.to_string(), max_tokens, self.model, Some(config)).await?;
        Ok(response.response)
    }
    
    fn name(&self) -> &str {
        "Single-step API"
    }
    
    fn requires_api_key(&self) -> bool {
        true
    }
}

/// Gets API key from config or environment
pub fn get_api_key(config: &AppConfig) -> Result<String> {
    // Try config first
    if let Some(key) = &config.openai_api_key {
        validate_api_key(Some(key))?;
        return Ok(key.clone());
    }

    // Try environment variable
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        validate_api_key(Some(&key))?;
        return Ok(key);
    }

    bail!(
        "OpenAI API key not found. Set via:\n\
         1. git-ai config set openai-api-key <key>\n\
         2. OPENAI_API_KEY environment variable"
    )
}

/// Validates an API key is present and valid
pub fn validate_api_key(key: Option<&str>) -> Result<&str> {
    match key {
        None => bail!("OpenAI API key not configured"),
        Some(k) if k.is_empty() || k == "<PLACE HOLDER FOR YOUR API KEY>" => {
            bail!("Invalid or placeholder API key")
        }
        Some(k) => Ok(k),
    }
}

/// Main entry point: Try strategies in order until one succeeds
pub async fn generate_with_fallback(
    diff: &str,
    config: &AppConfig,
) -> Result<String> {
    // Build strategy list
    let mut strategies: Vec<Box<dyn GenerationStrategy>> = Vec::new();
    
    // Try API strategies first if we have a key
    if get_api_key(config).is_ok() {
        if let Ok(multi_step) = MultiStepAPIStrategy::new(config) {
            strategies.push(Box::new(multi_step));
        }
    }
    
    // Always include local fallback
    strategies.push(Box::new(LocalMultiStepStrategy));
    
    // Single-step as final API fallback
    if let Ok(single_step) = SingleStepAPIStrategy::new(config) {
        strategies.push(Box::new(single_step));
    }
    
    let mut errors = Vec::new();
    
    for strategy in strategies {
        log::info!("Attempting generation with: {}", strategy.name());
        
        match strategy.generate(diff, config).await {
            Ok(message) => {
                log::info!("Successfully generated with: {}", strategy.name());
                return Ok(message);
            }
            Err(e) => {
                let error_msg = e.to_string();
                log::warn!("{} failed: {}", strategy.name(), error_msg);
                
                // Don't retry on auth errors
                if error_msg.contains("invalid_api_key") 
                   || error_msg.contains("Invalid API key") 
                   || error_msg.contains("Incorrect API key") {
                    return Err(e);
                }
                
                errors.push((strategy.name().to_string(), error_msg));
            }
        }
    }
    
    // All strategies failed
    let error_summary = errors
        .iter()
        .map(|(name, err)| format!("  - {}: {}", name, err))
        .collect::<Vec<_>>()
        .join("\n");
    
    bail!(
        "All generation strategies failed:\n{}",
        error_summary
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_fallback_with_no_api_key() {
        // Should fall back to local strategy
        let config = AppConfig::default();
        let result = generate_with_fallback("diff --git a/test.txt b/test.txt\n+Hello World", &config).await;
        // Should succeed with local strategy or fail with a reasonable error
        assert!(result.is_ok() || result.err().unwrap().to_string().contains("No files"));
    }
    
    #[test]
    fn test_validate_api_key() {
        assert!(validate_api_key(None).is_err());
        assert!(validate_api_key(Some("")).is_err());
        assert!(validate_api_key(Some("<PLACE HOLDER FOR YOUR API KEY>")).is_err());
        assert!(validate_api_key(Some("valid-key")).is_ok());
    }
    
    #[test]
    fn test_get_api_key_from_config() {
        let mut config = AppConfig::default();
        config.openai_api_key = Some("test-key".to_string());
        assert!(get_api_key(&config).is_ok());
    }
}