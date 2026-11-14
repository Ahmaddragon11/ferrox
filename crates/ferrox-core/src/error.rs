// SPDX-License-Identifier: MIT OR Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FerroxError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("engine error: {0}")]
    Engine(String),

    #[error("update error: {0}")]
    Update(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, FerroxError>;
