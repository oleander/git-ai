//! Commit message candidate generation and selection.

use super::scoring::ImpactScore;

pub struct Candidate {
    pub message: String,
    pub style: CandidateStyle,
}

pub enum CandidateStyle {
    Action,     // "Add authentication"
    Component,  // "auth: implementation"
    Impact,     // "New feature for authentication"
}

pub fn generate_candidates(
    scored_files: &[ImpactScore],
    max_length: usize,
) -> Vec<Candidate> {
    // This will be moved from multi_step_analysis.rs generate_commit_messages
    // For now, delegate to the old implementation
    let files_with_scores: Vec<crate::multi_step_analysis::FileWithScore> = scored_files
        .iter()
        .map(|impact_score| crate::multi_step_analysis::FileWithScore {
            file_path: impact_score.file_path.clone(),
            operation_type: impact_score.operation.clone(),
            lines_added: impact_score.analysis.lines_added,
            lines_removed: impact_score.analysis.lines_removed,
            file_category: impact_score.analysis.category.as_str().to_string(),
            summary: impact_score.analysis.summary.clone(),
            impact_score: impact_score.score,
        })
        .collect();

    let generate_result = crate::multi_step_analysis::generate_commit_messages(files_with_scores, max_length);
    
    // Convert to new Candidate format
    generate_result
        .candidates
        .into_iter()
        .enumerate()
        .map(|(i, message)| {
            let style = match i % 3 {
                0 => CandidateStyle::Action,
                1 => CandidateStyle::Component,
                _ => CandidateStyle::Impact,
            };
            Candidate { message, style }
        })
        .collect()
}

pub fn select_best_candidate(candidates: &[Candidate]) -> Option<String> {
    // For now, select the first candidate (action-focused)
    candidates.first().map(|c| c.message.clone())
}