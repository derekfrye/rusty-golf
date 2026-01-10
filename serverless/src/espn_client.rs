#![cfg(target_arch = "wasm32")]

use futures::{StreamExt, TryStreamExt, stream};
use rusty_golf_core::error::CoreError;
use rusty_golf_core::espn::EspnApiClient;
use rusty_golf_core::espn::processing::{merge_statistics_with_scores, process_json_to_statistics};
use rusty_golf_core::model::{PlayerJsonResponse, Scores};
use rusty_golf_core::storage::Storage;
use serde::Deserialize;
use std::collections::HashMap;
use worker::{Fetch, Url};

use crate::storage::ServerlessStorage;

pub struct ServerlessEspnClient {
    storage: ServerlessStorage,
}

const ESPN_FETCH_FANOUT: usize = 6;

impl ServerlessEspnClient {
    #[must_use]
    pub fn new(storage: ServerlessStorage) -> Self {
        Self { storage }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl EspnApiClient for ServerlessEspnClient {
    async fn get_json_from_espn(
        &self,
        scores: &[Scores],
        year: i32,
        event_id: i32,
    ) -> Result<PlayerJsonResponse, CoreError> {
        let fetches = stream::iter(scores.iter().map(|score| {
            let espn_id = score.espn_id;
            let eup_id = score.eup_id;
            async move {
                let url = format!(
                    "https://site.web.api.espn.com/apis/site/v2/sports/golf/pga/leaderboard/{}/playersummary?season={}&player={}",
                    event_id, year, espn_id
                );
                let url = Url::parse(&url).map_err(|e| CoreError::Network(e.to_string()))?;
                let mut resp = Fetch::Url(url)
                    .send()
                    .await
                    .map_err(|e| CoreError::Network(e.to_string()))?;
                let json: HashMap<String, serde_json::Value> = resp
                    .json()
                    .await
                    .map_err(|e| CoreError::Network(e.to_string()))?;

                if json.contains_key("rounds") {
                    Ok::<Option<(i64, HashMap<String, serde_json::Value>)>, CoreError>(Some((
                        eup_id, json,
                    )))
                } else {
                    Ok::<Option<(i64, HashMap<String, serde_json::Value>)>, CoreError>(None)
                }
            }
        }))
        .buffer_unordered(ESPN_FETCH_FANOUT)
        .try_collect::<Vec<_>>()
        .await?;

        let mut player_response = PlayerJsonResponse {
            data: Vec::new(),
            eup_ids: Vec::new(),
        };

        for maybe_response in fetches {
            if let Some((eup_id, json)) = maybe_response {
                player_response.data.push(json);
                player_response.eup_ids.push(eup_id);
            }
        }

        Ok(player_response)
    }

    async fn fallback_scores(&self, event_id: i32) -> Result<Option<Vec<Scores>>, CoreError> {
        let cache_key = ServerlessStorage::espn_cache_key(event_id);
        let cached: serde_json::Value = match self.storage.r2_get_json(&cache_key).await {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };
        Ok(self.parse_cached_scores(event_id, cached).await)
    }
}

#[derive(Deserialize)]
struct CachedScoresPayload {
    score_struct: Vec<Scores>,
}

impl ServerlessEspnClient {
    async fn parse_cached_scores(
        &self,
        event_id: i32,
        cached: serde_json::Value,
    ) -> Option<Vec<Scores>> {
        if let Ok(payload) = serde_json::from_value::<CachedScoresPayload>(cached.clone()) {
            return Some(payload.score_struct);
        }
        if let Ok(scores) = serde_json::from_value::<Vec<Scores>>(cached.clone()) {
            return Some(scores);
        }
        if let Ok(player_json) = serde_json::from_value::<PlayerJsonResponse>(cached) {
            let active_golfers = self.storage.get_golfers_for_event(event_id).await.ok()?;
            let stats = process_json_to_statistics(&player_json).ok()?;
            return merge_statistics_with_scores(&stats, &active_golfers).ok();
        }
        None
    }
}
