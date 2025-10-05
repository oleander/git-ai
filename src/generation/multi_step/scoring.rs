//! Impact scoring for analyzed files.

use super::analysis::FileAnalysis;

pub struct ImpactScore {
  pub file_path: String,
  pub operation: String,
  pub analysis:  FileAnalysis,
  pub score:     f32
}

pub fn calculate_impact_scores(files: Vec<(String, String, FileAnalysis)>) -> Vec<ImpactScore> {
  // This will be moved from multi_step_analysis.rs calculate_impact_scores
  // For now, delegate to the old implementation
  let files_data: Vec<crate::multi_step_analysis::FileDataForScoring> = files
    .iter()
    .map(|(path, operation, analysis)| {
      crate::multi_step_analysis::FileDataForScoring {
        file_path:      path.clone(),
        operation_type: operation.clone(),
        lines_added:    analysis.lines_added,
        lines_removed:  analysis.lines_removed,
        file_category:  analysis.category.as_str().to_string(),
        summary:        analysis.summary.clone()
      }
    })
    .collect();

  let score_result = crate::multi_step_analysis::calculate_impact_scores(files_data);

  score_result
    .files_with_scores
    .into_iter()
    .map(|file_with_score| {
      ImpactScore {
        file_path: file_with_score.file_path,
        operation: file_with_score.operation_type,
        analysis:  FileAnalysis {
          lines_added:   file_with_score.lines_added,
          lines_removed: file_with_score.lines_removed,
          category:      super::analysis::FileCategory::from(file_with_score.file_category.as_str()),
          summary:       file_with_score.summary
        },
        score:     file_with_score.impact_score
      }
    })
    .collect()
}

#[allow(dead_code)]
fn calculate_single_score(operation: &str, analysis: &FileAnalysis) -> f32 {
  // Implement locally for now to avoid private function call
  let operation_weight = match operation {
    "added" => 0.3,
    "modified" => 0.2,
    "deleted" => 0.25,
    "renamed" => 0.1,
    "binary" => 0.05,
    _ => 0.2 // default for unknown operations
  };

  let category_weight = match analysis.category {
    super::analysis::FileCategory::Source => 0.4,
    super::analysis::FileCategory::Test => 0.2,
    super::analysis::FileCategory::Config => 0.25,
    super::analysis::FileCategory::Build => 0.3,
    super::analysis::FileCategory::Docs => 0.1,
    super::analysis::FileCategory::Binary => 0.05
  };

  let total_lines = analysis.lines_added + analysis.lines_removed;
  let lines_normalized = (total_lines as f32 / 100.0).min(1.0);

  (operation_weight + category_weight + lines_normalized).min(1.0)
}
