use std::path::PathBuf;

#[cfg(not(mock))]
use clap::Parser;
use git2::Oid;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
  pub commit_msg_file: PathBuf,

  #[clap(required = false)]
  pub commit_type: Option<String>,

  #[clap(required = false)]
  pub sha1: Option<Oid>
}
