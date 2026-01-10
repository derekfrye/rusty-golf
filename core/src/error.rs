use crate::storage::StorageError;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum CoreError {
    #[error("db error: {0}")]
    Db(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("{0}")]
    Other(String),
}

impl From<StorageError> for CoreError {
    fn from(err: StorageError) -> Self {
        Self::Db(err.to_string())
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(err: serde_json::Error) -> Self {
        Self::Parse(err.to_string())
    }
}

impl From<std::io::Error> for CoreError {
    fn from(err: std::io::Error) -> Self {
        Self::Other(err.to_string())
    }
}

impl From<String> for CoreError {
    fn from(err: String) -> Self {
        Self::Other(err)
    }
}

impl From<&str> for CoreError {
    fn from(err: &str) -> Self {
        Self::Other(err.to_string())
    }
}
