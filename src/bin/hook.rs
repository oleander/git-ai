// Hook: prepare-commit-msg

#![feature(assert_matches)]

use std::io::{Read, Write};
use std::time::Duration;
use std::path::PathBuf;
use std::fs::File;

#[cfg(not(mock))]
use ai::hook::Args;
use git2::{DiffFormat, DiffOptions, Oid, Repository, Tree};
use indicatif::{ProgressBar, ProgressStyle};
use anyhow::{bail, Context, Result};
use ai::chat::generate_commit;
use lazy_static::lazy_static;
use dotenv_codegen::dotenv;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();
  ai::hook::run(Args::parse()).await
}
