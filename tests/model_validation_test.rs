use std::str::FromStr;

use ai::model::Model;

#[test]
fn test_valid_model_names() {
  // Test all supported model names
  assert_eq!(Model::from_str("gpt-4.1").unwrap(), Model::GPT41);
  assert_eq!(Model::from_str("gpt-4.1-mini").unwrap(), Model::GPT41Mini);
  assert_eq!(Model::from_str("gpt-4.1-nano").unwrap(), Model::GPT41Nano);
  assert_eq!(Model::from_str("gpt-4.5").unwrap(), Model::GPT45);
}

#[test]
fn test_case_insensitive_parsing() {
  // Test that model names are case-insensitive
  assert_eq!(Model::from_str("GPT-4.1").unwrap(), Model::GPT41);
  assert_eq!(Model::from_str("Gpt-4.1-Mini").unwrap(), Model::GPT41Mini);
  assert_eq!(Model::from_str("GPT-4.1-NANO").unwrap(), Model::GPT41Nano);
  assert_eq!(Model::from_str("gPt-4.5").unwrap(), Model::GPT45);
}

#[test]
fn test_whitespace_handling() {
  // Test that leading/trailing whitespace is trimmed
  assert_eq!(Model::from_str("  gpt-4.1  ").unwrap(), Model::GPT41);
  assert_eq!(Model::from_str("\tgpt-4.1-mini\n").unwrap(), Model::GPT41Mini);
}

#[test]
fn test_deprecated_model_backward_compat() {
  // Test that deprecated models map to their GPT-4.1 equivalents
  // These should succeed but log warnings
  assert_eq!(Model::from_str("gpt-4").unwrap(), Model::GPT41);
  assert_eq!(Model::from_str("gpt-4o").unwrap(), Model::GPT41);
  assert_eq!(Model::from_str("gpt-4o-mini").unwrap(), Model::GPT41Mini);
  assert_eq!(Model::from_str("gpt-3.5-turbo").unwrap(), Model::GPT41Mini);
}

#[test]
fn test_arbitrary_model_name_accepted() {
  // Unknown model strings are now accepted and carried through verbatim via Other.
  let model = Model::from_str("llama3.1:8b").unwrap();
  assert_eq!(model, Model::Other("llama3.1:8b".to_string()));
  // The real string is preserved (original case, no normalization).
  assert_eq!(model.as_str(), "llama3.1:8b");
  assert_eq!(model.to_string(), "llama3.1:8b");

  // Case is preserved for arbitrary names (local/ollama names can be case-sensitive).
  let mixed = Model::from_str("MyCustom-Model").unwrap();
  assert_eq!(mixed, Model::Other("MyCustom-Model".to_string()));
}

#[test]
fn test_empty_model_name_rejected() {
  // Empty / whitespace-only names are the only rejected input.
  assert!(Model::from_str("").is_err());
  assert!(Model::from_str("   ").is_err());
}

#[test]
fn test_unknown_model_fallback_carries_through() {
  // From<&str> no longer falls back to default for unknown models; it carries them.
  let model = Model::from("custom-model");
  assert_eq!(model, Model::Other("custom-model".to_string()));
}

#[test]
fn test_model_display() {
  // Test that models display correctly
  assert_eq!(Model::GPT41.to_string(), "gpt-4.1");
  assert_eq!(Model::GPT41Mini.to_string(), "gpt-4.1-mini");
  assert_eq!(Model::GPT41Nano.to_string(), "gpt-4.1-nano");
  assert_eq!(Model::GPT45.to_string(), "gpt-4.5");
}

#[test]
fn test_model_as_str() {
  // Test the as_str() method
  assert_eq!(Model::GPT41.as_str(), "gpt-4.1");
  assert_eq!(Model::GPT41Mini.as_str(), "gpt-4.1-mini");
  assert_eq!(Model::GPT41Nano.as_str(), "gpt-4.1-nano");
  assert_eq!(Model::GPT45.as_str(), "gpt-4.5");
}

#[test]
fn test_model_as_ref() {
  // Test the AsRef<str> implementation
  fn takes_str_ref<S: AsRef<str>>(s: S) -> String {
    s.as_ref().to_string()
  }

  assert_eq!(takes_str_ref(Model::GPT41), "gpt-4.1");
  assert_eq!(takes_str_ref(Model::GPT41Mini), "gpt-4.1-mini");
}

#[test]
fn test_model_from_string() {
  // Test conversion from String
  let s = String::from("gpt-4.1");
  assert_eq!(Model::from(s), Model::GPT41);

  let s = String::from("gpt-4.1-mini");
  assert_eq!(Model::from(s), Model::GPT41Mini);
}

#[test]
fn test_default_model() {
  // Test that the default model is GPT41Mini (F3).
  assert_eq!(Model::default(), Model::GPT41Mini);
}

#[test]
fn test_unknown_model_tokenizer_and_context_fallback() {
  // Unknown models fall back to the cl100k_base tokenizer and a sane default context size.
  let model = Model::from_str("some-unknown-model-xyz").unwrap();
  // Tokenizer fallback works (non-zero count for non-empty text, no panic).
  let count = model
    .count_tokens("hello world, this is a token count test")
    .unwrap();
  assert!(count > 0, "unknown model should still count tokens via cl100k_base fallback");
  // Context size falls back to 4096 for unrecognized models.
  assert_eq!(model.context_size(), 4096);
}
