#![allow(dead_code)]

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CfdError {
    #[error("{0}")]
    Message(String),
    #[error("http {status}")]
    HttpStatus { status: u16 },
    #[error("transport error: {message}")]
    Transport { message: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl CfdError {
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    pub fn transport(message: impl Into<String>) -> Self {
        Self::Transport {
            message: message.into(),
        }
    }
}
