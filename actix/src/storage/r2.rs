use chrono::NaiveDateTime;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use rusty_golf_core::storage::{EventDetails, Storage, StorageError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use super::r2_types::R2EventDetails;
use crate::model::{RefreshSource, Scores, ScoresAndLastRefresh, Statistic};

pub use super::r2_config::R2StorageConfig;
pub use super::r2_signing::{MissingSigner, S3Signer, SigV4Signer};

#[derive(Clone)]
pub struct R2Storage {
    client: reqwest::Client,
    config: R2StorageConfig,
    signer: Arc<dyn S3Signer>,
}

impl R2Storage {
    #[must_use]
    pub fn new(config: R2StorageConfig, signer: Arc<dyn S3Signer>) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
            signer,
        }
    }

    fn object_url(&self, key: &str) -> String {
        let base = self.config.endpoint.trim_end_matches('/');
        let bucket = self.config.bucket.trim_matches('/');
        format!("{base}/{bucket}/{key}")
    }

    async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let url = self.object_url(key);
        let headers = self.signer.sign("GET", &url, HeaderMap::new(), None)?;
        let resp = self
            .client
            .get(url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| StorageError::new(e.to_string()))?;

        if resp.status().as_u16() == 404 {
            return Ok(None);
        }

        if !resp.status().is_success() {
            return Err(StorageError::new(format!(
                "R2 GET failed with status {}",
                resp.status()
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| StorageError::new(e.to_string()))?;
        Ok(Some(bytes.to_vec()))
    }

    async fn put_object(&self, key: &str, body: Vec<u8>) -> Result<(), StorageError> {
        let url = self.object_url(key);
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let headers = self.signer.sign("PUT", &url, headers, Some(&body))?;

        let resp = self
            .client
            .put(url)
            .headers(headers)
            .body(body)
            .send()
            .await
            .map_err(|e| StorageError::new(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(StorageError::new(format!(
                "R2 PUT failed with status {}",
                resp.status()
            )));
        }

        Ok(())
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, StorageError> {
        let Some(bytes) = self.get_object(key).await? else {
            return Ok(None);
        };
        let parsed =
            serde_json::from_slice::<T>(&bytes).map_err(|e| StorageError::new(e.to_string()))?;
        Ok(Some(parsed))
    }

    async fn put_json<T: Serialize>(&self, key: &str, value: &T) -> Result<(), StorageError> {
        let body = serde_json::to_vec(value).map_err(|e| StorageError::new(e.to_string()))?;
        self.put_object(key, body).await
    }
}

#[async_trait::async_trait]
impl Storage for R2Storage {
    async fn get_event_details(&self, event_id: i32) -> Result<EventDetails, StorageError> {
        let key = Self::event_key(event_id);
        let details = self
            .get_json::<R2EventDetails>(&key)
            .await?
            .ok_or_else(|| StorageError::new("event details not found"))?;

        Ok(EventDetails {
            event_name: details.event_name,
            score_view_step_factor: details.score_view_step_factor,
            refresh_from_espn: details.refresh_from_espn,
            end_date: details.end_date,
        })
    }

    async fn get_golfers_for_event(&self, event_id: i32) -> Result<Vec<Scores>, StorageError> {
        let golfers_key = Self::golfers_key(event_id);
        if let Some(golfers) = self.get_json::<Vec<Scores>>(&golfers_key).await? {
            return Ok(golfers);
        }

        let scores_key = Self::scores_key(event_id);
        if let Some(scores) = self.get_json::<ScoresAndLastRefresh>(&scores_key).await? {
            let golfers = scores
                .score_struct
                .iter()
                .map(|score| Scores {
                    detailed_statistics: Statistic {
                        eup_id: score.eup_id,
                        rounds: Vec::new(),
                        round_scores: Vec::new(),
                        tee_times: Vec::new(),
                        holes_completed_by_round: Vec::new(),
                        line_scores: Vec::new(),
                        total_score: 0,
                    },
                    ..score.clone()
                })
                .collect();
            return Ok(golfers);
        }

        Err(StorageError::new("golfers not found"))
    }

    async fn get_player_step_factors(
        &self,
        event_id: i32,
    ) -> Result<HashMap<(i64, String), f32>, StorageError> {
        let scores_key = Self::scores_key(event_id);
        let scores = self
            .get_json::<ScoresAndLastRefresh>(&scores_key)
            .await?
            .ok_or_else(|| StorageError::new("scores not found"))?;

        let step_factors = scores
            .score_struct
            .iter()
            .filter_map(|score| {
                let step = score.score_view_step_factor?;
                Some(((score.espn_id, score.bettor_name.clone()), step))
            })
            .collect();

        Ok(step_factors)
    }

    async fn get_scores(
        &self,
        event_id: i32,
        source: RefreshSource,
    ) -> Result<ScoresAndLastRefresh, StorageError> {
        let key = Self::scores_key(event_id);
        let mut scores = self
            .get_json::<ScoresAndLastRefresh>(&key)
            .await?
            .ok_or_else(|| StorageError::new("scores not found"))?;
        scores.last_refresh_source = source;
        Ok(scores)
    }

    async fn store_scores(&self, event_id: i32, scores: &[Scores]) -> Result<(), StorageError> {
        let payload = ScoresAndLastRefresh {
            score_struct: scores.to_vec(),
            last_refresh: chrono::Utc::now().naive_utc(),
            last_refresh_source: RefreshSource::Espn,
        };
        self.put_json(&Self::scores_key(event_id), &payload).await
    }

    async fn event_and_scores_already_in_db(
        &self,
        event_id: i32,
        max_age_seconds: i64,
    ) -> Result<bool, StorageError> {
        let key = Self::scores_key(event_id);
        let Some(scores) = self.get_json::<ScoresAndLastRefresh>(&key).await? else {
            return Ok(false);
        };

        let now = chrono::Utc::now().naive_utc();
        let last_refresh: NaiveDateTime = scores.last_refresh;
        // Match SQL behavior: compare cache age in days.
        let diff = now.signed_duration_since(last_refresh);
        Ok(diff.num_days() >= max_age_seconds)
    }
}
