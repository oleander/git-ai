#![feature(lazy_cell)]

pub mod git;
pub mod chat;

use git::Repo;
use std::os::unix::process::CommandExt;
use std::process::{exit, Command};
use dotenv::dotenv;
use anyhow::Result;
use clap::Parser;
use log::error;
