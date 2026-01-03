use async_trait::async_trait;
use rusty_golf_core::storage::{EventDetails, Storage, StorageError};
use sql_middleware::middleware::ConfigAndPool;
use std::collections::HashMap;

use crate::model::{
    RefreshSource, Scores, ScoresAndLastRefresh, event_and_scores_already_in_db, get_event_details,
    get_golfers_from_db, get_player_step_factors, get_scores_from_db, store_scores_in_db,
};

pub mod r2;

pub use r2::{MissingSigner, R2Storage, R2StorageConfig, S3Signer, SigV4Signer};

#[derive(Clone)]
pub struct SqlStorage {
    config_and_pool: ConfigAndPool,
}

impl SqlStorage {
    #[must_use]
    pub fn new(config_and_pool: ConfigAndPool) -> Self {
        Self { config_and_pool }
    }

    #[must_use]
    pub fn config_and_pool(&self) -> &ConfigAndPool {
        &self.config_and_pool
    }
}

#[async_trait]
impl Storage for SqlStorage {
    async fn get_event_details(&self, event_id: i32) -> Result<EventDetails, StorageError> {
        let details = get_event_details(&self.config_and_pool, event_id)
            .await
            .map_err(|e| StorageError::new(e.to_string()))?;
        Ok(EventDetails {
            event_name: details.event_name,
            score_view_step_factor: details.score_view_step_factor,
            refresh_from_espn: details.refresh_from_espn,
        })
    }

    async fn get_golfers_for_event(&self, event_id: i32) -> Result<Vec<Scores>, StorageError> {
        get_golfers_from_db(&self.config_and_pool, event_id)
            .await
            .map_err(|e| StorageError::new(e.to_string()))
    }

    async fn get_player_step_factors(
        &self,
        event_id: i32,
    ) -> Result<HashMap<(i64, String), f32>, StorageError> {
        get_player_step_factors(&self.config_and_pool, event_id)
            .await
            .map_err(|e| StorageError::new(e.to_string()))
    }

    async fn get_scores(
        &self,
        event_id: i32,
        source: RefreshSource,
    ) -> Result<ScoresAndLastRefresh, StorageError> {
        get_scores_from_db(&self.config_and_pool, event_id, source)
            .await
            .map_err(|e| StorageError::new(e.to_string()))
    }

    async fn store_scores(&self, event_id: i32, scores: &[Scores]) -> Result<(), StorageError> {
        store_scores_in_db(&self.config_and_pool, event_id, scores)
            .await
            .map_err(|e| StorageError::new(e.to_string()))
    }

    async fn event_and_scores_already_in_db(
        &self,
        event_id: i32,
        max_age_seconds: i64,
    ) -> Result<bool, StorageError> {
        event_and_scores_already_in_db(&self.config_and_pool, event_id, max_age_seconds)
            .await
            .map_err(|e| StorageError::new(e.to_string()))
    }
}
