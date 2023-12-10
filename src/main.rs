use config::Config;

// APP_OPENAI_API_KEY="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"
// APP_MODEL="gpt-4-1106-preview"
// APP_MAX_DIFF_TOKENS=5000
// APP_MAX_LENGTH=72
// APP_LANGUAGE=en
// APP_TIMEOUT=30

#[derive(Debug, Default, serde_derive::Deserialize, PartialEq, Eq)]
struct AppConfig {
  openai_api_key:  Option<String>,
  model:           Option<String>,
  max_diff_tokens: Option<u32>,
  max_length:      Option<u32>,
  language:        Option<String>,
  timeout:         Option<u32>
}

fn main() {
  dotenv::dotenv().ok();
  let config = Config::builder()
    .add_source(config::Environment::with_prefix("APP").try_parsing(true))
    .build()
    .unwrap();

  println!("{:?}", config);
  let app: AppConfig = config.try_deserialize().unwrap();

  println!("{:?}", app);
  // assert_eq!(
}
