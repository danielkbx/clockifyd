#![allow(dead_code)]

pub mod client;
pub mod config;
pub mod entry;
pub mod list_columns;
pub mod login;
pub mod logout;
pub mod project;
pub mod tag;
pub mod task;
pub mod timer;
pub mod whoami;
pub mod workspace;

use crate::error::CfdError;

pub fn not_implemented(command: &str) -> Result<(), CfdError> {
    Err(CfdError::message(format!(
        "command not implemented yet: {command}"
    )))
}
