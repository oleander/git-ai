use std::env;

use config::{Config, File, FileFormat};

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
  language:        String,
  timeout:         Option<u32>
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  dotenv::dotenv().ok();
  let home_dir = env::var("HOME").expect("Failed to find HOME directory");
  let bin_name = env::var("CARGO_PKG_NAME").unwrap_or("git-ai".to_owned());
  let config_dir = format!("{home_dir}/.config/{bin_name}");
  let config_file = format!("{config_dir}/config.ini");

  let config = Config::builder()
    .add_source(config::Environment::with_prefix("APP").try_parsing(true))
    .add_source(config::File::new(config_file.as_str(), FileFormat::Ini))
    .set_default("language", "en")?
    .build()
    .unwrap();

  println!("{:?}", config);
  let app: AppConfig = config.try_deserialize().unwrap();

  println!("{:?}", app);
  // assert_eq!(
    Ok(())
}
