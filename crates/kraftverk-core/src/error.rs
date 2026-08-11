//! Shared error types.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),

    #[error("unsupported: {0}")]
    Unsupported(String),

    #[error("unsupported hardware: {0}")]
    UnsupportedHardware(String),

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("platform error: {0}")]
    Platform(String),

    #[error("benchmark error: {0}")]
    Benchmark(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("rollback required: {0}")]
    RollbackRequired(String),

    #[error("interrupted experiment detected: {0}")]
    Interrupted(String),

    #[error("statistics error: {0}")]
    Statistics(String),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }

    pub fn unsupported(s: impl Into<String>) -> Self {
        Self::Unsupported(s.into())
    }

    pub fn unsupported_hardware(s: impl Into<String>) -> Self {
        Self::UnsupportedHardware(s.into())
    }
}
