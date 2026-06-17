//! Invariant tests pinning the commit-message generation prompt contract.
//!
//! These guard against silent regressions in the prompt fine-tuning: they assert that
//! every prompt (the inline system prompts for the multi-step pipeline and the
//! single-step fallback template in `resources/prompt.md`) still carries its critical
//! invariants -- the structured-output / function-calling requirement, imperative-mood
//! guidance, and (for the template) the `{{max_length}}` mustache placeholder. They also
//! pin the tool/function names, which live in the function-calling schema rather than in
//! the prompt text. Editing the natural-language instructions is fine; breaking the
//! contract these tests encode is not.

use ai::function_calling::create_commit_function_tool;
use ai::multi_step_analysis::{create_analyze_function_tool, create_generate_function_tool, create_score_function_tool};
use ai::multi_step_integration::{ANALYZE_SYSTEM_PROMPT, COMMIT_SYSTEM_PROMPT, GENERATE_SYSTEM_PROMPT, SCORE_SYSTEM_PROMPT};

/// The raw, unrendered single-step fallback template, embedded the same way `commit.rs`
/// embeds it. Asserting against this (not the rendered output) is what verifies the
/// `{{max_length}}` placeholder still exists -- the rendered template replaces it.
const RAW_PROMPT_MD: &str = include_str!("../resources/prompt.md");

fn lower(s: &str) -> String {
  s.to_lowercase()
}

#[test]
fn analyze_prompt_keeps_core_invariants() {
  let p = lower(ANALYZE_SYSTEM_PROMPT);
  // Imperative-mood guidance.
  assert!(p.contains("imperative"), "analyze prompt must instruct imperative mood");
  // Anti-hallucination guardrail.
  assert!(
    p.contains("not invent") || p.contains("do not invent"),
    "analyze prompt must forbid inventing changes"
  );
  // Structured-output requirement (route the answer through the function tool).
  assert!(p.contains("function"), "analyze prompt must require returning via the function");
}

#[test]
fn score_prompt_keeps_core_invariants() {
  let p = lower(SCORE_SYSTEM_PROMPT);
  assert!(p.contains("impact"), "score prompt must mention impact scoring");
  assert!(p.contains("function"), "score prompt must require returning via the function");
}

#[test]
fn generate_prompt_keeps_core_invariants() {
  let p = lower(GENERATE_SYSTEM_PROMPT);
  assert!(p.contains("imperative"), "generate prompt must instruct imperative mood");
  // Length budget must be honored.
  assert!(
    p.contains("character limit") || p.contains("limit"),
    "generate prompt must mention the length budget"
  );
  assert!(p.contains("function"), "generate prompt must require returning via the function");
}

#[test]
fn commit_prompt_keeps_core_invariants() {
  let p = lower(COMMIT_SYSTEM_PROMPT);
  assert!(p.contains("imperative"), "commit prompt must instruct imperative mood");
  // It must drive the `commit` function specifically.
  assert!(p.contains("commit function"), "commit prompt must reference the commit function");
  assert!(p.contains("limit"), "commit prompt must mention the character limit");
}

#[test]
fn fallback_template_keeps_max_length_placeholder() {
  // Must assert on the RAW template: rendering substitutes a number for the placeholder,
  // so checking rendered output would verify nothing.
  assert!(
    RAW_PROMPT_MD.contains("{{max_length}}"),
    "resources/prompt.md must preserve the {{{{max_length}}}} mustache placeholder"
  );
}

#[test]
fn fallback_template_keeps_function_and_mood_invariants() {
  let p = lower(RAW_PROMPT_MD);
  // The single-step fallback still uses function calling -- it must not be rewritten into
  // a plain text-output prompt.
  assert!(
    p.contains("`commit` function") || p.contains("commit function"),
    "prompt.md must require the commit function"
  );
  // Imperative-mood guidance.
  assert!(p.contains("imperative"), "prompt.md must instruct imperative mood");
  // Anti-hallucination guardrail.
  assert!(
    p.contains("not invent") || p.contains("do not invent"),
    "prompt.md must forbid inventing changes"
  );
}

#[test]
fn tool_names_are_pinned() {
  // The tool/function names are part of the structured-output contract and live in the
  // schema, not the prompt text. Pin them here so a rename can't slip through.
  assert_eq!(create_analyze_function_tool().unwrap().function.name, "analyze");
  assert_eq!(create_score_function_tool().unwrap().function.name, "score");
  assert_eq!(create_generate_function_tool().unwrap().function.name, "generate");
  assert_eq!(create_commit_function_tool(None).unwrap().function.name, "commit");
}
