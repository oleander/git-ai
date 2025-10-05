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
pub use analysis::{FileAnalysis, analyze_file, analyze_file_via_api};
pub use scoring::{calculate_impact_scores, ImpactScore};
pub use candidates::{generate_candidates, select_best_candidate};

/// Represents a parsed file from the git diff
#[derive(Debug)]
pub struct ParsedFile {
    pub path: String,
    pub operation: String,
    pub diff_content: String,
}

/// Parse git diff into individual files
pub fn parse_diff(diff_content: &str) -> Result<Vec<ParsedFile>> {
    let old_files = crate::multi_step_integration::parse_diff(diff_content)?;
    Ok(old_files.into_iter().map(|f| ParsedFile {
        path: f.path,
        operation: f.operation,
        diff_content: f.diff_content,
    }).collect())
}

/// Call the analyze function via OpenAI
async fn call_analyze_function(client: &Client<OpenAIConfig>, model: &str, file: &ParsedFile) -> Result<serde_json::Value> {
    // Convert our ParsedFile to the old format
    let old_file = crate::multi_step_integration::ParsedFile {
        path: file.path.clone(),
        operation: file.operation.clone(),
        diff_content: file.diff_content.clone(),
    };
    crate::multi_step_integration::call_analyze_function(client, model, &old_file).await
}

/// Call the score function via OpenAI  
async fn call_score_function(
    client: &Client<OpenAIConfig>, 
    model: &str, 
    files_data: Vec<crate::multi_step_analysis::FileDataForScoring>
) -> Result<Vec<crate::multi_step_analysis::FileWithScore>> {
    crate::multi_step_integration::call_score_function(client, model, files_data).await
}

/// Call the generate function via OpenAI
async fn call_generate_function(
    client: &Client<OpenAIConfig>, 
    model: &str, 
    scored_files: Vec<crate::multi_step_analysis::FileWithScore>, 
    max_length: usize
) -> Result<serde_json::Value> {
    crate::multi_step_integration::call_generate_function(client, model, scored_files, max_length).await
}

/// Main entry point for multi-step generation with API
pub async fn generate_with_api(
    client: &Client<OpenAIConfig>,
    model: &str,
    diff: &str,
    max_length: Option<usize>,
) -> Result<String> {
    use futures::future::join_all;
    use crate::multi_step_analysis::FileDataForScoring;
    use crate::debug_output;

    log::info!("Starting multi-step commit message generation");

    // Initialize multi-step debug session
    if let Some(session) = debug_output::debug_session() {
        session.init_multi_step_debug();
    }

    // Parse the diff to extract individual files
    let parsed_files = parse_diff(diff)?;
    log::info!("Parsed {} files from diff", parsed_files.len());

    // Track files parsed in debug session
    if let Some(session) = debug_output::debug_session() {
        session.set_total_files_parsed(parsed_files.len());
    }

    // Step 1: Analyze each file individually in parallel
    log::debug!("Analyzing {} files in parallel", parsed_files.len());

    // Create futures for all file analyses
    let analysis_futures: Vec<_> = parsed_files
        .iter()
        .map(|file| {
            let file_path = file.path.clone();
            let operation = file.operation.clone();
            async move {
                log::debug!("Analyzing file: {file_path}");
                let start_time = std::time::Instant::now();
                let payload = format!("{{\"file_path\": \"{file_path}\", \"operation_type\": \"{operation}\", \"diff_content\": \"...\"}}");

                let result = call_analyze_function(client, model, file).await;
                let duration = start_time.elapsed();
                (file, result, duration, payload)
            }
        })
        .collect();

    // Execute all analyses in parallel
    let analysis_results = join_all(analysis_futures).await;

    // Process results and handle errors
    let mut file_analyses = Vec::new();

    for (i, (file, result, duration, payload)) in analysis_results.into_iter().enumerate() {
        match result {
            Ok(analysis) => {
                log::debug!("Successfully analyzed file {}: {}", i, file.path);

                // Extract structured analysis data for debug
                let analysis_result = crate::multi_step_analysis::FileAnalysisResult {
                    lines_added:   analysis["lines_added"].as_u64().unwrap_or(0) as u32,
                    lines_removed: analysis["lines_removed"].as_u64().unwrap_or(0) as u32,
                    file_category: analysis["file_category"]
                        .as_str()
                        .unwrap_or("source")
                        .to_string(),
                    summary:       analysis["summary"].as_str().unwrap_or("").to_string()
                };

                // Record in debug session
                if let Some(session) = debug_output::debug_session() {
                    session.add_file_analysis_debug(file.path.clone(), file.operation.clone(), analysis_result.clone(), duration, payload);
                }

                file_analyses.push((file, analysis));
            }
            Err(e) => {
                // Check if it's an API key error - if so, propagate it immediately
                let error_str = e.to_string();
                if error_str.contains("invalid_api_key") || error_str.contains("Incorrect API key") || error_str.contains("Invalid API key") {
                    return Err(e);
                }
                log::warn!("Failed to analyze file {}: {}", file.path, e);
                // Continue with other files even if one fails
            }
        }
    }

    if file_analyses.is_empty() {
        anyhow::bail!("Failed to analyze any files");
    }

    // Step 2: Calculate impact scores
    let files_data: Vec<FileDataForScoring> = file_analyses
        .iter()
        .map(|(file, analysis)| {
            FileDataForScoring {
                file_path:      file.path.clone(),
                operation_type: file.operation.clone(),
                lines_added:    analysis["lines_added"].as_u64().unwrap_or(0) as u32,
                lines_removed:  analysis["lines_removed"].as_u64().unwrap_or(0) as u32,
                file_category:  analysis["file_category"]
                    .as_str()
                    .unwrap_or("source")
                    .to_string(),
                summary:        analysis["summary"].as_str().unwrap_or("").to_string()
            }
        })
        .collect();

    log::debug!("Calculating impact scores for {} files", files_data.len());
    let start_time = std::time::Instant::now();
    let scored_files = call_score_function(client, model, files_data).await?;
    let duration = start_time.elapsed();

    // Record scoring debug info
    if let Some(session) = debug_output::debug_session() {
        let payload = format!("{{\"files_count\": {}, \"scoring_method\": \"api\"}}", scored_files.len());
        session.set_score_debug(scored_files.clone(), duration, payload);
    }

    log::debug!("Successfully scored {} files", scored_files.len());

    // Step 3: Generate commit message using the scored files
    log::debug!("Generating commit message from scored files");
    let start_time = std::time::Instant::now();
    let commit_result = call_generate_function(client, model, scored_files, max_length.unwrap_or(72)).await?;
    let duration = start_time.elapsed();

    // Record generate debug info
    if let Some(session) = debug_output::debug_session() {
        session.record_timing("generate", duration);
    }

    // Extract the commit message from the JSON response
    let message = commit_result["candidates"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No commit message candidates in response"))?;

    log::info!("Multi-step generation completed successfully");
    Ok(message.to_string())
}

/// Simplified multi-step commit message generation using OpenAI directly
pub async fn generate_simple(
    client: &Client<OpenAIConfig>,
    model: &str,
    diff_content: &str,
    max_length: Option<usize>,
) -> Result<String> {
    // Delegate to the existing simple multi-step implementation
    crate::simple_multi_step::generate_commit_message_simple(client, model, diff_content, max_length).await
}

/// Main entry point for local multi-step generation (no API)
pub fn generate_local(
    diff: &str,
    max_length: Option<usize>,
) -> Result<String> {
    use crate::multi_step_analysis::{analyze_file, calculate_impact_scores, generate_commit_messages, FileDataForScoring};
    use crate::debug_output;

    log::info!("Starting local multi-step commit message generation");

    // Parse the diff
    let parsed_files = parse_diff(diff)?;

    // Track files parsed in debug session
    if let Some(session) = debug_output::debug_session() {
        session.set_total_files_parsed(parsed_files.len());
    }

    // Step 1: Analyze each file
    let mut files_data = Vec::new();
    for file in parsed_files {
        let analysis = analyze_file(&file.path, &file.diff_content, &file.operation);
        files_data.push(FileDataForScoring {
            file_path:      file.path,
            operation_type: file.operation,
            lines_added:    analysis.lines_added,
            lines_removed:  analysis.lines_removed,
            file_category:  analysis.file_category,
            summary:        analysis.summary
        });
    }

    // Step 2: Calculate impact scores
    let score_result = calculate_impact_scores(files_data);

    // Step 3: Generate commit messages
    let generate_result = generate_commit_messages(score_result.files_with_scores, max_length.unwrap_or(72));

    // Return the first candidate
    generate_result.candidates
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No commit message candidates generated"))
}