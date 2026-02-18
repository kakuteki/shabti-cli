use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShabtiError {
    #[error("storage error: {0}")]
    Storage(String),

    #[error("index error: {0}")]
    Index(String),

    #[error("embedding error: {0}")]
    Embedding(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("duplicate entry: {0}")]
    Duplicate(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),
}

pub type ShabtiResult<T> = Result<T, ShabtiError>;
