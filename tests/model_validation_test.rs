use ai::model::Model;
use std::str::FromStr;

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
fn test_invalid_model_name() {
  // Test that invalid model names return an error
  let result = Model::from_str("does-not-exist");
  assert!(result.is_err());
  assert!(result
    .unwrap_err()
    .to_string()
    .contains("Invalid model name"));
}

#[test]
fn test_invalid_model_fallback() {
  // Test that From<&str> falls back to default for invalid models
  let model = Model::from("invalid-model");
  assert_eq!(model, Model::default());
  assert_eq!(model, Model::GPT41);
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

  assert_eq!(takes_str_ref(&Model::GPT41), "gpt-4.1");
  assert_eq!(takes_str_ref(&Model::GPT41Mini), "gpt-4.1-mini");
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
  // Test that the default model is GPT41
  assert_eq!(Model::default(), Model::GPT41);
}
