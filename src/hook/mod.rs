// Hook: prepare-commit-msg
pub mod traits;

use std::path::PathBuf;

use git2::{Oid, Repository};
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use dotenv_codegen::dotenv;
use clap::Parser;
use thiserror::Error;
use traits::*;

use crate::chat::{generate_commit, ChatError};
use crate::hook::traits::{FilePath, PatchRepository};
use crate::config;
use traits::*;