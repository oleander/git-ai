// Hook: prepare-commit-msg
pub mod traits;

use std::path::PathBuf;

use traits::*;

use crate::chat::{generate_commit, ChatError};
use crate::hook::traits::{FilePath, PatchRepository};
use crate::config;
