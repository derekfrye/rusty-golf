use crate::model::{RefreshSource, Scores, ScoresAndLastRefresh};
use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub struct EventDetails {
    pub event_name: String,
    pub score_view_step_factor: f32,
    pub refresh_from_espn: i64,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StorageError {
    message: String,
}

impl StorageError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for StorageError {}

impl From<String> for StorageError {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for StorageError {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait Storage: Send + Sync {
    async fn get_event_details(&self, event_id: i32) -> Result<EventDetails, StorageError>;
    async fn get_golfers_for_event(&self, event_id: i32) -> Result<Vec<Scores>, StorageError>;
    async fn get_player_step_factors(
        &self,
        event_id: i32,
    ) -> Result<HashMap<(i64, String), f32>, StorageError>;
    async fn get_scores(
        &self,
        event_id: i32,
        source: RefreshSource,
    ) -> Result<ScoresAndLastRefresh, StorageError>;
    async fn store_scores(&self, event_id: i32, scores: &[Scores]) -> Result<(), StorageError>;
    async fn event_and_scores_already_in_db(
        &self,
        event_id: i32,
        max_age_seconds: i64,
    ) -> Result<bool, StorageError>;
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait Storage {
    async fn get_event_details(&self, event_id: i32) -> Result<EventDetails, StorageError>;
    async fn get_golfers_for_event(&self, event_id: i32) -> Result<Vec<Scores>, StorageError>;
    async fn get_player_step_factors(
        &self,
        event_id: i32,
    ) -> Result<HashMap<(i64, String), f32>, StorageError>;
    async fn get_scores(
        &self,
        event_id: i32,
        source: RefreshSource,
    ) -> Result<ScoresAndLastRefresh, StorageError>;
    async fn store_scores(&self, event_id: i32, scores: &[Scores]) -> Result<(), StorageError>;
    async fn event_and_scores_already_in_db(
        &self,
        event_id: i32,
        max_age_seconds: i64,
    ) -> Result<bool, StorageError>;
}
