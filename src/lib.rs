#![forbid(unsafe_code)]

pub mod app;
pub mod codec;
pub mod event;
pub mod model;
pub mod retrieval;
pub mod store;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("corruption: {0}")]
    Corrupt(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("NarrowScope: short query exceeds 64 candidates; specify time or slot")]
    NarrowScope,
    #[error("model: {0}")]
    Model(String),
    #[error("ContextTooSmall: complete system/question exceeds input budget")]
    ContextTooSmall,
    #[error("cancelled")]
    Cancelled,
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
