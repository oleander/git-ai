use std::io::{self, Write};

use anyhow::Result;
use console::Emoji;
use dialoguer::{Input, Select};
use ai::model::Model;

use crate::config;

pub fn run() -> Result<()> {
  println!("\n{} Welcome to git-ai configuration wizard! {}", Emoji("👋", ":-)"), Emoji("✨", ":-)"));
  println!("Let's set up everything you need to get started.\n");

  // Get OpenAI API key
  let api_key: String = Input::new()
    .with_prompt("Please enter your OpenAI API key")
    .interact_text()?;
  config::run_openai_api_key(api_key)?;

  // Select model
  let models = Model::variants();
  let model_names: Vec<String> = models.iter().map(|m| m.to_string()).collect();
  let model_idx = Select::new()
    .with_prompt("Select the OpenAI model to use")
    .default(0)
    .items(&model_names)
    .interact()?;
  config::run_model(model_names[model_idx].clone())?;

  // Set max tokens
  let max_tokens: usize = Input::new()
    .with_prompt("Maximum tokens per request")
    .default(2024)
    .interact_text()?;
  config::run_max_tokens(max_tokens)?;

  // Set max commit length
  let max_commit_length: usize = Input::new()
    .with_prompt("Maximum commit message length")
    .default(72)
    .interact_text()?;
  config::run_max_commit_length(max_commit_length)?;

  println!("\n{} Configuration complete! You're all set to use git-ai.", Emoji("🎉", ":-)"));
  Ok(())
}

pub fn needs_setup() -> bool {
  let app = config::App::new().ok();
  match app {
    Some(config) => config.openai_api_key.as_deref() == Some("<PLACE HOLDER FOR YOUR API KEY>"),
    None => true
  }
}
