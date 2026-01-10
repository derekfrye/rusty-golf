#![cfg(target_arch = "wasm32")]

use async_trait::async_trait;
use chrono::Utc;
use rusty_golf_core::model::score::Statistic;
use rusty_golf_core::model::{RefreshSource, Scores, ScoresAndLastRefresh};
use rusty_golf_core::storage::{EventDetails, Storage, StorageError};
use std::collections::HashMap;

use crate::storage::ServerlessStorage;
use super::storage_helpers::{format_rfc3339, parse_rfc3339};
use super::storage_types::{
    EventDetailsDoc, GolferAssignment, LastRefreshDoc, PlayerFactorEntry, SeededAtDoc,
};

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl Storage for ServerlessStorage {
    async fn get_event_details(&self, event_id: i32) -> Result<EventDetails, StorageError> {
        let key = Self::kv_event_details_key(event_id);
        let doc: EventDetailsDoc = self.kv_get_json(&key).await?;
        Ok(EventDetails {
            event_name: doc.event_name,
            score_view_step_factor: doc.score_view_step_factor,
            refresh_from_espn: doc.refresh_from_espn,
            end_date: doc.end_date,
        })
    }

    async fn get_golfers_for_event(&self, event_id: i32) -> Result<Vec<Scores>, StorageError> {
        let key = Self::kv_golfers_key(event_id);
        let assignments: Vec<GolferAssignment> = self.kv_get_json(&key).await?;
        Ok(assignments
            .into_iter()
            .map(|assignment| Scores {
                eup_id: assignment.eup_id,
                espn_id: assignment.espn_id,
                golfer_name: assignment.golfer_name,
                bettor_name: assignment.bettor_name,
                detailed_statistics: Statistic {
                    eup_id: assignment.eup_id,
                    rounds: Vec::new(),
                    round_scores: Vec::new(),
                    tee_times: Vec::new(),
                    holes_completed_by_round: Vec::new(),
                    line_scores: Vec::new(),
                    total_score: 0,
                },
                group: assignment.group,
                score_view_step_factor: assignment.score_view_step_factor,
            })
            .collect())
    }

    async fn get_player_step_factors(
        &self,
        event_id: i32,
    ) -> Result<HashMap<(i64, String), f32>, StorageError> {
        let key = Self::kv_player_factors_key(event_id);
        let entries: Vec<PlayerFactorEntry> = self.kv_get_json(&key).await?;
        Ok(entries
            .into_iter()
            .map(|entry| ((entry.golfer_espn_id, entry.bettor_name), entry.step_factor))
            .collect())
    }

    async fn get_scores(
        &self,
        event_id: i32,
        source: RefreshSource,
    ) -> Result<ScoresAndLastRefresh, StorageError> {
        let key = Self::scores_key(event_id);
        let mut scores: ScoresAndLastRefresh = self.r2_get_json(&key).await?;
        scores.last_refresh_source = source;
        Ok(scores)
    }

    async fn store_scores(&self, event_id: i32, scores: &[Scores]) -> Result<(), StorageError> {
        let now = Utc::now().naive_utc();
        let payload = ScoresAndLastRefresh {
            score_struct: scores.to_vec(),
            last_refresh: now,
            last_refresh_source: RefreshSource::Espn,
        };
        let key = Self::scores_key(event_id);
        self.r2_put_json(&key, &payload).await?;

        let last_refresh = LastRefreshDoc {
            ts: format_rfc3339(now),
            source: RefreshSource::Espn,
        };
        let kv_key = Self::kv_last_refresh_key(event_id);
        self.kv_put_json(&kv_key, &last_refresh).await?;

        let seeded_at = SeededAtDoc {
            seeded_at: format_rfc3339(now),
        };
        let seeded_key = Self::kv_seeded_at_key(event_id, "last_refresh");
        self.kv_put_json(&seeded_key, &seeded_at).await?;
        Ok(())
    }

    async fn event_and_scores_already_in_db(
        &self,
        event_id: i32,
        max_age_seconds: i64,
    ) -> Result<bool, StorageError> {
        if max_age_seconds <= 0 {
            return Ok(false);
        }
        let details_key = Self::kv_event_details_key(event_id);
        if self
            .kv
            .get(&details_key)
            .text()
            .await
            .ok()
            .flatten()
            .is_none()
        {
            return Ok(false);
        }

        let last_refresh_key = Self::kv_last_refresh_key(event_id);
        let last_refresh: LastRefreshDoc = match self.kv_get_json(&last_refresh_key).await {
            Ok(val) => val,
            Err(_) => return Ok(false),
        };

        let last_refresh_ts =
            parse_rfc3339(&last_refresh.ts).map_err(|e| StorageError::new(e.to_string()))?;
        let now = Utc::now().naive_utc();
        let diff = now.signed_duration_since(last_refresh_ts);
        Ok(diff.num_seconds() <= max_age_seconds)
    }
}
