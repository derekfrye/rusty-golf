use std::fmt;

#[derive(Debug, Clone)]
pub enum AppError {
    Db(String),
    Network(String),
    Parse(String),
    NotFound(String),
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Db(s) => write!(f, "db error: {s}"),
            AppError::Network(s) => write!(f, "network error: {s}"),
            AppError::Parse(s) => write!(f, "parse error: {s}"),
            AppError::NotFound(s) => write!(f, "not found: {s}"),
            AppError::Other(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<sql_middleware::SqlMiddlewareDbError> for AppError {
    fn from(e: sql_middleware::SqlMiddlewareDbError) -> Self {
        Self::Db(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        Self::Network(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::Parse(e.to_string())
    }
}

impl From<String> for AppError {
    fn from(e: String) -> Self {
        Self::Other(e)
    }
}

impl From<&str> for AppError {
    fn from(e: &str) -> Self {
        Self::Other(e.to_string())
    }
}
