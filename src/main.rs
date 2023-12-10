use std::env;

use config::{Config, FileFormat};

#[derive(Debug, Default, serde_derive::Deserialize, PartialEq, Eq)]
struct App {
  openai_api_key:  String,
  model:           String,
  max_diff_tokens: usize,
  max_length:      usize,
  language:        String,
  timeout:         usize
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  dotenv::dotenv().ok();
  let config_path = home::home_dir().unwrap().join(".config/git-ai/config.ini");

  let config = Config::builder()
    .add_source(config::Environment::with_prefix("APP").try_parsing(true))
    .add_source(config::File::new(config_path.to_str().unwrap(), FileFormat::Ini))
    .set_default("language", "en")?
    .set_default("timeout", 30)?
    .set_default("max_length", 72)?
    .set_default("max_diff_tokens", 5000)?
    .set_default("model", "gpt-4-1106-preview")?
    .build()?;

  let app: App = config.try_deserialize()?;

  println!("{:?}", app);
  // assert_eq!(
  Ok(())
}
