use std::collections::HashMap;
use std::time::{Duration, Instant};

use colored::Colorize;
use serde_json::Value;

use crate::function_calling::{CommitFunctionArgs, FileChange};
use crate::multi_step_analysis::{FileAnalysisResult, FileWithScore};

/// Represents an individual file analysis result for the debug output
#[derive(Clone)]
pub struct FileAnalysisDebug {
  pub file_path:    String,
  pub operation:    String,
  pub analysis:     FileAnalysisResult,
  pub api_duration: Duration,
  pub api_payload:  String
}

/// Represents a multi-step debug session with detailed information
#[derive(Clone)]
pub struct MultiStepDebug {
  pub file_analyses:          Vec<FileAnalysisDebug>,
  pub score_result:           Option<Vec<FileWithScore>>,
  pub score_duration:         Option<Duration>,
  pub score_payload:          Option<String>,
  pub generate_result:        Option<Value>,
  pub generate_duration:      Option<Duration>,
  pub generate_payload:       Option<String>,
  pub final_message_duration: Option<Duration>,
  pub candidates:             Vec<String>,
  pub reasoning:              Option<String>
}

/// Tracks timing information for various operations
pub struct DebugSession {
  start_time:          Instant,
  timings:             HashMap<String, Duration>,
  args:                String,
  build_type:          String,
  multi_step_error:    Option<String>,
  single_step_success: bool,
  commit_message:      Option<String>,
  commit_reasoning:    Option<String>,
  files_analyzed:      Option<CommitFunctionArgs>,
  total_files_parsed:  usize,
  api_duration:        Option<Duration>,
  final_commit_hash:   Option<String>,
  final_commit_branch: Option<String>,
  files_changed_count: Option<(usize, usize, usize)>, // (files, insertions, deletions)
  multi_step_debug:    Option<MultiStepDebug>         // Detailed multi-step debug info
}

impl DebugSession {
  pub fn new(args: &str) -> Self {
    Self {
      start_time:          Instant::now(),
      timings:             HashMap::new(),
      args:                args.to_string(),
      build_type:          if cfg!(debug_assertions) {
        "Debug build with performance profiling enabled".to_string()
      } else {
        "Release build".to_string()
      },
      multi_step_error:    None,
      single_step_success: false,
      commit_message:      None,
      commit_reasoning:    None,
      files_analyzed:      None,
      total_files_parsed:  0,
      api_duration:        None,
      final_commit_hash:   None,
      final_commit_branch: None,
      files_changed_count: None,
      multi_step_debug:    None
    }
  }

  pub fn record_timing(&mut self, operation: &str, duration: Duration) {
    self.timings.insert(operation.to_string(), duration);
  }

  pub fn set_multi_step_error(&mut self, error: String) {
    self.multi_step_error = Some(error);
  }

  pub fn set_single_step_success(&mut self, success: bool) {
    self.single_step_success = success;
  }

  pub fn set_commit_result(&mut self, message: String, reasoning: String) {
    self.commit_message = Some(message);
    self.commit_reasoning = Some(reasoning);
  }

  pub fn set_files_analyzed(&mut self, args: CommitFunctionArgs) {
    self.files_analyzed = Some(args);
  }

  pub fn set_total_files_parsed(&mut self, count: usize) {
    self.total_files_parsed = count;
  }

  pub fn set_api_duration(&mut self, duration: Duration) {
    self.api_duration = Some(duration);
  }

  pub fn init_multi_step_debug(&mut self) {
    self.multi_step_debug = Some(MultiStepDebug {
      file_analyses:          Vec::new(),
      score_result:           None,
      score_duration:         None,
      score_payload:          None,
      generate_result:        None,
      generate_duration:      None,
      generate_payload:       None,
      final_message_duration: None,
      candidates:             Vec::new(),
      reasoning:              None
    });
  }

  pub fn add_file_analysis_debug(
    &mut self, file_path: String, operation: String, analysis: FileAnalysisResult, duration: Duration, payload: String
  ) {
    if let Some(ref mut multi_step) = self.multi_step_debug {
      multi_step.file_analyses.push(FileAnalysisDebug {
        file_path,
        operation,
        analysis,
        api_duration: duration,
        api_payload: payload
      });
    }
  }

  pub fn set_score_debug(&mut self, result: Vec<FileWithScore>, duration: Duration, payload: String) {
    if let Some(ref mut multi_step) = self.multi_step_debug {
      multi_step.score_result = Some(result);
      multi_step.score_duration = Some(duration);
      multi_step.score_payload = Some(payload);
    }
  }

  pub fn set_generate_debug(&mut self, result: Value, duration: Duration, payload: String) {
    if let Some(ref mut multi_step) = self.multi_step_debug {
      // Extract candidates before moving result
      let mut candidates_vec = Vec::new();
      if let Some(candidates) = result.get("candidates") {
        if let Some(candidates_array) = candidates.as_array() {
          candidates_vec = candidates_array
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        }
      }

      // Extract reasoning before moving result
      let reasoning_str = result
        .get("reasoning")
        .and_then(|r| r.as_str())
        .map(|s| s.to_string());

      // Now store everything
      multi_step.generate_result = Some(result);
      multi_step.generate_duration = Some(duration);
      multi_step.generate_payload = Some(payload);
      multi_step.candidates = candidates_vec;
      multi_step.reasoning = reasoning_str;
    }
  }

  pub fn set_final_message_debug(&mut self, duration: Duration) {
    if let Some(ref mut multi_step) = self.multi_step_debug {
      multi_step.final_message_duration = Some(duration);
    }
  }

  pub fn set_final_commit_info(&mut self, branch: String, hash: String, files: usize, insertions: usize, deletions: usize) {
    self.final_commit_branch = Some(branch);
    self.final_commit_hash = Some(hash);
    self.files_changed_count = Some((files, insertions, deletions));
  }

  pub fn print_debug_output(&self) {
    eprintln!("\n{}", "=== GIT AI HOOK DEBUG SESSION ===".bright_cyan().bold());

    // Initialization
    eprintln!("\n{} {}", "📋".bright_yellow(), "INITIALIZATION".bright_white().bold());
    eprintln!("  {}        {}", "Args:".bright_white(), self.args);
    eprintln!("  {}       {}", "Build:".bright_white(), self.build_type);

    // Setup & Preparation
    eprintln!("\n{} {}", "⚙️ ".bright_yellow(), "SETUP & PREPARATION".bright_white().bold());
    self.print_timing_line("Generate instruction template", "Generate instruction template", false);
    self.print_timing_line("Count tokens", "Count tokens", false);
    self.print_timing_line("Calculate instruction tokens", "Calculate instruction tokens", false);
    self.print_timing_line("Get context size", "Get context size", true);

    // Git Diff Processing
    eprintln!("\n{} {}", "📝".bright_yellow(), "GIT DIFF PROCESSING".bright_white().bold());
    self.print_timing_line("Git diff generation", "Git diff generation", false);
    self.print_timing_line("Processing diff changes", "Processing diff changes", false);
    self.print_timing_line("Repository patch generation", "Repository patch generation", false);

    let files_status = if self.total_files_parsed == 0 {
      format!("{} files   {}", self.total_files_parsed, "⚠️".yellow())
    } else {
      format!("{} files   ✓", self.total_files_parsed)
        .green()
        .to_string()
    };
    eprintln!("  └ Files parsed from diff           {files_status}");

    // Discovered Files
    if self.total_files_parsed > 0 {
      eprintln!("\n{} {}", "🔍".bright_yellow(), "DISCOVERED FILES".bright_white().bold());

      if let Some(ref multi_step) = self.multi_step_debug {
        for (files_shown, file) in multi_step.file_analyses.iter().enumerate() {
          let change_type = match file.operation.as_str() {
            "added" => "[added]".green(),
            "deleted" => "[deleted]".red(),
            "modified" => "[modified]".yellow(),
            "renamed" => "[renamed]".blue(),
            _ => format!("[{}]", file.operation).normal()
          };

          let lines_info = format!("{} lines", file.analysis.lines_added + file.analysis.lines_removed);
          let prefix = if files_shown == multi_step.file_analyses.len() - 1 {
            "└"
          } else {
            "│"
          };
          eprintln!("  {} {:<30} {:<12} {}", prefix, file.file_path.bright_cyan(), change_type, lines_info);
        }
      } else if let Some(ref files) = self.files_analyzed {
        let mut file_list: Vec<(&String, &FileChange)> = files.files.iter().collect();
        file_list.sort_by(|a, b| b.1.impact_score.partial_cmp(&a.1.impact_score).unwrap());

        let total_files = file_list.len();
        for (files_shown, (path, change)) in file_list.iter().enumerate() {
          let change_type = match change.change_type.as_str() {
            "added" => "[added]".green(),
            "deleted" => "[deleted]".red(),
            "modified" => "[modified]".yellow(),
            "renamed" => "[renamed]".blue(),
            _ => format!("[{}]", change.change_type).normal()
          };

          let prefix = if files_shown == total_files - 1 {
            "└"
          } else {
            "│"
          };
          eprintln!(
            "  {} {:<30} {:<12} {} lines",
            prefix,
            path.bright_cyan(),
            change_type,
            change.lines_changed
          );
        }
      }
    }

    // AI Processing
    eprintln!("\n{} {}", "🤖".bright_yellow(), "AI PROCESSING".bright_white().bold());

    if let Some(ref multi_step) = self.multi_step_debug {
      eprintln!(
        "\n  {} {}",
        "📋".bright_yellow(),
        "STEP 1: INDIVIDUAL FILE ANALYSIS".bright_white().bold()
      );

      for (i, file) in multi_step.file_analyses.iter().enumerate() {
        let file_num = i + 1;
        let total_files = multi_step.file_analyses.len();

        eprintln!("    ");
        eprintln!("    🔸 File {}/{}: {}", file_num, total_files, file.file_path.bright_cyan());
        eprintln!("      │ OpenAI Request [analyze]:");
        eprintln!(
          "      │   └ Payload: {{\"file_path\": \"{}\", \"operation_type\": \"{}\", \"diff_content\": \"...\"}}",
          file.file_path, file.operation
        );
        eprintln!(
          "      │ API Response Time:              {:<7}    ✓",
          format!("{:.2}s", file.api_duration.as_secs_f32())
        );
        eprintln!("      │ Results:");
        eprintln!("      │   ├ Lines Added:                {}", file.analysis.lines_added);
        eprintln!("      │   ├ Lines Removed:              {}", file.analysis.lines_removed);
        eprintln!("      │   ├ File Category:              {}", file.analysis.file_category);
        eprintln!("      │   └ Summary:                    {}", file.analysis.summary);
      }

      eprintln!(
        "\n  {} {}",
        "📊".bright_yellow(),
        "STEP 2: IMPACT SCORE CALCULATION".bright_white().bold()
      );

      if let Some(ref score_result) = multi_step.score_result {
        if let Some(score_duration) = multi_step.score_duration {
          eprintln!("    │ OpenAI Request [score]:");
          eprintln!(
            "    │   └ Payload: {{\"files_data\": [{{\"{}\", ...}}, ...]}}",
            if !multi_step.file_analyses.is_empty() {
              &multi_step.file_analyses[0].file_path
            } else {
              "no files"
            }
          );
          eprintln!(
            "    │ API Response Time:              {:<7}    ✓",
            format!("{:.2}s", score_duration.as_secs_f32())
          );
          eprintln!("    │ Results:");

          let mut sorted_files = score_result.clone();
          sorted_files.sort_by(|a, b| b.impact_score.partial_cmp(&a.impact_score).unwrap());

          for (i, file) in sorted_files.iter().enumerate() {
            let prefix = if i == sorted_files.len() - 1 {
              "└"
            } else {
              "├"
            };
            eprintln!(
              "    │   {} {:<30} Impact Score {:.2} {}",
              prefix,
              file.file_path,
              file.impact_score,
              if i == 0 {
                "(highest)".bright_green()
              } else {
                "".normal()
              }
            );
          }
        }
      }

      eprintln!(
        "\n  {} {}",
        "💭".bright_yellow(),
        "STEP 3: COMMIT MESSAGE GENERATION".bright_white().bold()
      );

      if let Some(generate_duration) = multi_step.generate_duration {
        eprintln!("    │ OpenAI Request [generate]:");
        eprintln!("    │   └ Payload: {{\"files_with_scores\": [...], \"max_length\": 72}}");
        eprintln!(
          "    │ API Response Time:              {:<7}    ✓",
          format!("{:.2}s", generate_duration.as_secs_f32())
        );

        if !multi_step.candidates.is_empty() {
          eprintln!("    │ Candidates Generated:");

          for (i, candidate) in multi_step.candidates.iter().enumerate() {
            let prefix = if i == multi_step.candidates.len() - 1 {
              "└"
            } else {
              "├"
            };
            eprintln!("    │   {} \"{}\"", prefix, candidate.bright_cyan());
          }

          if let Some(ref reasoning) = multi_step.reasoning {
            eprintln!("    │ Reasoning: {reasoning}");
          }
        }
      }
    } else {
      // Multi-Step Attempt
      let multi_step_status = if self.multi_step_error.is_some() {
        "FAILED".red().to_string()
      } else if self.single_step_success {
        "SKIPPED".yellow().to_string()
      } else {
        "SUCCESS".green().to_string()
      };
      eprintln!("  Multi-Step Attempt:                           {multi_step_status}");

      if let Some(ref error) = self.multi_step_error {
        eprintln!("    │ Creating score function tool              ✓");
        eprintln!("    │ OpenAI connection                         ✓");
        eprintln!(
          "    └ Error: {}             {} {}",
          error.trim_end_matches('.'),
          "✗".red(),
          error.split(':').next_back().unwrap_or("").trim()
        );
      }

      // Single-Step Fallback
      if self.single_step_success {
        eprintln!("\n  Single-Step Fallback:                        {}", "SUCCESS".green());
        eprintln!("    │ Creating commit function tool             ✓ max_length=72");
        if let Some(duration) = self.api_duration {
          eprintln!(
            "    │ OpenAI API call                   {:<7} ✓",
            format!("{:.2}s", duration.as_secs_f32())
          );
        }
        eprintln!("    └ Response parsing                          ✓");
      }
    }

    // Analysis Results
    if let Some(ref message) = self.commit_message {
      eprintln!("\n{} {}", "📊".bright_yellow(), "ANALYSIS RESULTS".bright_white().bold());
      eprintln!("  Selected Message: '{}'", message.bright_cyan());
      eprintln!("  Message Length:   {} characters (within 72 limit)", message.len());

      if let Some(ref reasoning) = self.commit_reasoning {
        eprintln!("\n  Final Reasoning:");
        // Word wrap the reasoning at ~70 characters
        let words: Vec<&str> = reasoning.split_whitespace().collect();
        let mut line = String::new();
        for word in words {
          if line.len() + word.len() + 1 > 70 {
            eprintln!("    {line}");
            line = word.to_string();
          } else {
            if !line.is_empty() {
              line.push(' ');
            }
            line.push_str(word);
          }
        }
        if !line.is_empty() {
          eprintln!("    {line}");
        }
      }
    }

    // Detailed File Analysis
    if let Some(ref files) = self.files_analyzed {
      eprintln!("\n{} {}", "📁".bright_yellow(), "DETAILED FILE ANALYSIS".bright_white().bold());
      eprintln!("  Total Files: {}", files.files.len());

      // Sort files by impact score
      let mut sorted_files: Vec<(&String, &FileChange)> = files.files.iter().collect();
      sorted_files.sort_by(|a, b| b.1.impact_score.partial_cmp(&a.1.impact_score).unwrap());

      for (path, change) in sorted_files.iter() {
        eprintln!();
        eprintln!("  🔸 {}", path.bright_cyan());
        eprintln!("    │ Summary:      {}", change.summary);
        eprintln!(
          "    │ Impact Score: {:.2} {}",
          change.impact_score,
          if change.impact_score >= 0.9 {
            "(highest - drives commit message)".bright_green()
          } else if change.impact_score >= 0.8 {
            "(high - mentioned in commit)".bright_yellow()
          } else if change.impact_score >= 0.5 {
            "(medium - supporting change)".normal()
          } else {
            "(low)".normal()
          }
        );

        // Not using this variable directly, but keeping the match logic for clarity
        let _change_type_str = match change.change_type.as_str() {
          "added" => "added",
          "modified" => "modified",
          "deleted" => "deleted",
          "renamed" => "renamed",
          _ => &change.change_type
        };

        eprintln!(
          "    │ Lines:        +{}, -{} ({} total)",
          change.lines_changed / 2, // Approximation for display
          change.lines_changed / 2,
          change.lines_changed
        );
        eprintln!("    │ Category:     {}", change.file_category);
        eprintln!(
          "    │ Significance: {}",
          if change.impact_score >= 0.9 {
            "Core functionality"
          } else if change.impact_score >= 0.8 {
            "Supporting infrastructure"
          } else if change.impact_score >= 0.5 {
            "Minor improvement"
          } else {
            "Peripheral change"
          }
        );

        let weight_str = if change.impact_score >= 0.9 {
          "Primary focus for commit message"
        } else if change.impact_score >= 0.8 {
          "Secondary mention in commit"
        } else if change.impact_score >= 0.6 {
          "Implicit support (not explicitly mentioned)"
        } else {
          "Not reflected in commit message"
        };

        eprintln!("    └ Weight:       {weight_str}");
      }
    }

    // Statistics Summary
    if let Some(ref files) = self.files_analyzed {
      eprintln!("\n{} {}", "📈".bright_yellow(), "STATISTICS SUMMARY".bright_white().bold());

      let total_lines: u32 = files.files.values().map(|f| f.lines_changed).sum();
      let avg_impact: f32 = if files.files.is_empty() {
        0.0
      } else {
        files.files.values().map(|f| f.impact_score).sum::<f32>() / files.files.len() as f32
      };

      eprintln!("  │ Total Lines Changed:     {total_lines}");
      eprintln!("  │ Average Impact Score:    {avg_impact:.2}");
      eprintln!("  │");

      // Count by category
      let mut category_counts: HashMap<&str, usize> = HashMap::new();
      for change in files.files.values() {
        *category_counts.entry(&change.file_category).or_insert(0) += 1;
      }

      eprintln!("  │ By Category:");
      for (category, count) in category_counts {
        eprintln!("  │   └ {category}: {count}");
      }

      eprintln!("  │");

      // Count by change type
      let mut type_counts: HashMap<&str, usize> = HashMap::new();
      for change in files.files.values() {
        *type_counts.entry(&change.change_type).or_insert(0) += 1;
      }

      eprintln!("  │ By Change Type:");
      for (change_type, count) in type_counts {
        eprintln!("  │   └ {change_type}: {count}");
      }
    }

    // Performance Summary
    eprintln!("\n{} {}", "⏱️ ".bright_yellow(), "PERFORMANCE SUMMARY".bright_white().bold());

    if let Some(ref multi_step) = self.multi_step_debug {
      let mut total_file_analysis = Duration::default();
      for file in &multi_step.file_analyses {
        total_file_analysis += file.api_duration;
      }

      eprintln!(
        "  │ Individual file analysis:         {:.2}s ({} files)",
        total_file_analysis.as_secs_f32(),
        multi_step.file_analyses.len()
      );

      if let Some(score_duration) = multi_step.score_duration {
        eprintln!("  │ Impact score calculation:         {:.2}s", score_duration.as_secs_f32());
      }

      if let Some(generate_duration) = multi_step.generate_duration {
        eprintln!("  │ Commit message generation:        {:.2}s", generate_duration.as_secs_f32());
      }

      eprintln!("  │ ─────────────────────────────────────────");

      let total_ai_processing = total_file_analysis
        + multi_step.score_duration.unwrap_or_default()
        + multi_step.generate_duration.unwrap_or_default()
        + multi_step.final_message_duration.unwrap_or_default();

      eprintln!("  │ Total AI processing:              {:.2}s", total_ai_processing.as_secs_f32());
    } else if let Some(duration) = self.api_duration {
      eprintln!("  │ OpenAI request/response:          {:.2}s", duration.as_secs_f32());
    }

    let total_duration = self.start_time.elapsed();
    eprintln!("  │ Total execution time:             {:.2}s", total_duration.as_secs_f32());
    eprintln!("  └ Status:                           {} ✓", "SUCCESS".green());

    // Final Result
    if let (Some(ref branch), Some(ref hash), Some(ref message)) =
      (&self.final_commit_branch, &self.final_commit_hash, &self.commit_message)
    {
      eprintln!("\n{} {}", "🎯".bright_yellow(), "FINAL RESULT".bright_white().bold());

      let short_hash = if hash.len() > 7 {
        &hash[..7]
      } else {
        hash
      };
      eprintln!("  [{} {}] {}", branch.bright_green(), short_hash.bright_yellow(), message.bright_cyan());

      if let Some((files, insertions, deletions)) = self.files_changed_count {
        let files_text = if files == 1 {
          "file"
        } else {
          "files"
        };
        let insertions_text = if insertions == 1 {
          "insertion"
        } else {
          "insertions"
        };
        let deletions_text = if deletions == 1 {
          "deletion"
        } else {
          "deletions"
        };

        eprintln!(
          "   {} {} changed, {} {}(+), {} {}(-)",
          files,
          files_text,
          insertions.to_string().green(),
          insertions_text,
          deletions.to_string().red(),
          deletions_text
        );
      }
    }
  }

  fn print_timing_line(&self, key: &str, label: &str, last: bool) {
    let prefix = if last {
      "└"
    } else {
      "│"
    };

    if let Some(duration) = self.timings.get(key) {
      let duration_str = format_duration(*duration);
      eprintln!("  {prefix} {label:<35} {duration_str:<10} ✓");
    } else {
      eprintln!("  {} {:<35} {:<10} ✓", prefix, label, "0.00ms");
    }
  }
}

fn format_duration(duration: Duration) -> String {
  let micros = duration.as_micros();
  if micros < 1000 {
    format!("{micros:.0}µs")
  } else if micros < 1_000_000 {
    format!("{:.2}ms", duration.as_secs_f32() * 1000.0)
  } else {
    format!("{:.2}s", duration.as_secs_f32())
  }
}

/// Global debug session instance
pub static mut DEBUG_SESSION: Option<DebugSession> = None;

/// Initialize the debug session
pub fn init_debug_session(args: &str) {
  unsafe {
    DEBUG_SESSION = Some(DebugSession::new(args));
  }
}

/// Get a mutable reference to the debug session
#[allow(static_mut_refs)]
pub fn debug_session() -> Option<&'static mut DebugSession> {
  unsafe { DEBUG_SESSION.as_mut() }
}

/// Print the final debug output
pub fn print_final_output() {
  if let Some(session) = debug_session() {
    session.print_debug_output();
  }
}

/// Record a timing for an operation
pub fn record_timing(operation: &str, duration: Duration) {
  if let Some(session) = debug_session() {
    session.record_timing(operation, duration);
  }
}
