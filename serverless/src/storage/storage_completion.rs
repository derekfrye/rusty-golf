#![cfg(target_arch = "wasm32")]

use chrono::Utc;
use rusty_golf_core::score::should_promote_completed;
use rusty_golf_core::storage::{Storage, StorageError};
use serde::Deserialize;
use worker::{Fetch, Url};

use super::storage_cache::derive_cache_ttls;
use super::storage_helpers::{format_rfc3339, parse_rfc3339};
use super::storage_types::{EventDetailsDoc, SeededAtDoc};
use crate::storage::ServerlessStorage;

const ESPN_SCOREBOARD_HEADER_URL: &str = "https://site.web.api.espn.com/apis/v2/scoreboard/header?sport=golf&league=pga&region=us&lang=en&contentorigin=espn";
const COMPLETION_PROMOTION_GRACE_DAYS: i64 = 5;

impl ServerlessStorage {
    pub(super) async fn cache_ttls_for_event(&self, event_id: i32) -> (u64, i64) {
        let details = self.get_event_details(event_id).await.ok();
        derive_cache_ttls(
            details.as_ref().and_then(|doc| doc.start_date.as_deref()),
            details.as_ref().and_then(|doc| doc.end_date.as_deref()),
            details.as_ref().map(|doc| doc.completed).unwrap_or(false),
        )
    }

    pub(super) async fn promote_completed_if_ready(
        &self,
        event_id: i32,
    ) -> Result<(), StorageError> {
        let details_key = Self::kv_event_details_key(event_id);
        let mut details: EventDetailsDoc = self.kv_get_json(&details_key).await?;
        if details.completed {
            return Ok(());
        }

        let now = Utc::now().naive_utc();
        if let Some(end_date) = details.end_date.as_deref()
            && let Ok(parsed) = parse_rfc3339(end_date)
            && now <= parsed + chrono::Duration::days(COMPLETION_PROMOTION_GRACE_DAYS)
        {
            return Ok(());
        }

        let Some((espn_completed, espn_end_date)) =
            self.fetch_event_completion_state(event_id).await?
        else {
            return Ok(());
        };
        let end_date = details.end_date.as_deref().or(espn_end_date.as_deref());
        if !should_promote_completed(espn_completed, end_date, now) {
            return Ok(());
        }

        details.completed = true;
        self.kv_put_json(&details_key, &details).await?;

        let seeded_at = SeededAtDoc {
            seeded_at: format_rfc3339(now),
        };
        let seeded_key = Self::kv_seeded_at_key(event_id, "details");
        let _ = self.kv_put_json(&seeded_key, &seeded_at).await;
        Ok(())
    }

    async fn fetch_event_completion_state(
        &self,
        event_id: i32,
    ) -> Result<Option<(bool, Option<String>)>, StorageError> {
        let url =
            Url::parse(ESPN_SCOREBOARD_HEADER_URL).map_err(|e| StorageError::new(e.to_string()))?;
        let mut response = Fetch::Url(url)
            .send()
            .await
            .map_err(|e| StorageError::new(e.to_string()))?;
        let payload: ScoreboardHeader = response
            .json()
            .await
            .map_err(|e| StorageError::new(e.to_string()))?;

        Ok(find_scoreboard_event(payload, event_id))
    }
}

fn find_scoreboard_event(
    payload: ScoreboardHeader,
    event_id: i32,
) -> Option<(bool, Option<String>)> {
    for sport in payload.sports {
        for league in sport.leagues {
            for event in league.events {
                if event.id.parse::<i32>().ok() != Some(event_id) {
                    continue;
                }
                let completed = event
                    .full_status
                    .as_ref()
                    .is_some_and(ScoreboardFullStatus::completed);
                return Some((completed, event.end_date));
            }
        }
    }
    None
}

#[derive(Debug, Deserialize)]
struct ScoreboardHeader {
    sports: Vec<ScoreboardSport>,
}

#[derive(Debug, Deserialize)]
struct ScoreboardSport {
    leagues: Vec<ScoreboardLeague>,
}

#[derive(Debug, Deserialize)]
struct ScoreboardLeague {
    events: Vec<ScoreboardEvent>,
}

#[derive(Debug, Deserialize)]
struct ScoreboardEvent {
    id: String,
    #[serde(rename = "endDate")]
    end_date: Option<String>,
    #[serde(rename = "fullStatus")]
    full_status: Option<ScoreboardFullStatus>,
}

#[derive(Debug, Deserialize)]
struct ScoreboardFullStatus {
    #[serde(default)]
    completed: bool,
    #[serde(rename = "type")]
    status_type: Option<ScoreboardStatusType>,
}

#[derive(Debug, Deserialize)]
struct ScoreboardStatusType {
    #[serde(default)]
    completed: bool,
}

impl ScoreboardFullStatus {
    fn completed(&self) -> bool {
        self.completed
            || self
                .status_type
                .as_ref()
                .is_some_and(|status| status.completed)
    }
}
