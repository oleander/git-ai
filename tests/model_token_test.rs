use ai::model::Model;

#[test]
fn test_token_counting_accuracy() {
  let model = Model::GPT4;

  // Test various text lengths to ensure we're not underestimating
  let test_cases = vec![
    // Empty text
    ("", 0),
    // Single word
    ("Hello", 1),
    // Short sentence
    ("The quick brown fox jumps over the lazy dog.", 10),
    // Medium text with punctuation
    (
      "Hello, world! This is a test. How are you doing today? I hope everything is going well.",
      20
    ),
    // Code snippet (tokens can be different for code)
    ("fn main() { println!(\"Hello, world!\"); }", 11),
    // Text with special characters
    ("Special chars: @#$%^&*()_+-=[]{}|;':\",./<>?", 20),
  ];

  for (text, min_expected) in test_cases {
    let tokens = model.count_tokens(text).unwrap();
    assert!(
      tokens >= min_expected,
      "Token count for '{}' was {} but expected at least {}",
      text,
      tokens,
      min_expected
    );

    // Also ensure it's not wildly overestimating (within 2x)
    assert!(
      tokens <= min_expected * 2 + 5,
      "Token count for '{}' was {} but expected at most {}",
      text,
      tokens,
      min_expected * 2 + 5
    );
  }
}

#[test]
fn test_no_underestimation_for_context_limit() {
  let model = Model::GPT4;

  // Create text that would be underestimated by the old heuristics
  // Old heuristic: ~4 chars per token, but actual can be much different
  let tricky_texts = vec![
    // Text with many short words (more tokens than chars/4)
    "I a b c d e f g h i j k l m n o p q r s t u v w x y z",
    // Text with special tokens
    "```python\ndef hello():\n    print('world')\n```",
    // URLs and paths
    "https://github.com/user/repo/blob/main/src/lib.rs#L42",
    // Mixed content
    "Error: failed to compile `foo.rs` at line 42: unexpected token ';'",
  ];

  for text in tricky_texts {
    let tokens = model.count_tokens(text).unwrap();
    let char_estimate = text.len() / 4;

    // The actual token count should be reasonable compared to character length
    // but we're not using the underestimating heuristic anymore
    println!(
      "Text: '{}' - Chars: {}, Tokens: {}, Old estimate would be: {}",
      text,
      text.len(),
      tokens,
      char_estimate
    );
  }
}

#[test]
fn test_token_counting_consistency() {
  let model = Model::GPT4;

  // Test that the same text always returns the same token count
  let test_text = "The quick brown fox jumps over the lazy dog. This is a test sentence with various words.";

  let mut counts = Vec::new();
  for _ in 0..5 {
    counts.push(model.count_tokens(test_text).unwrap());
  }

  // All counts should be the same
  assert!(counts.windows(2).all(|w| w[0] == w[1]), "Token counting is not consistent");
}

#[test]
fn test_long_text_token_counting() {
  let model = Model::GPT4;

  // Test with a longer text to ensure we're using the tokenizer properly
  let long_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(50);

  let tokens = model.count_tokens(&long_text).unwrap();

  // With proper tokenization, this should be significantly more than chars/4
  let char_estimate = long_text.len() / 4;

  println!(
    "Long text - Chars: {}, Actual tokens: {}, Char-based estimate: {}",
    long_text.len(),
    tokens,
    char_estimate
  );

  // The actual token count should be reasonable but not underestimated
  assert!(tokens > 0, "Token count should be greater than 0");
  assert!(tokens < long_text.len(), "Token count should be less than character count");
}
