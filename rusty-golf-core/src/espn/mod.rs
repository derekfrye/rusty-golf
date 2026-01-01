pub mod processing;

use async_trait::async_trait;
use crate::error::CoreError;
use crate::model::{PlayerJsonResponse, RefreshSource, Scores, ScoresAndLastRefresh};
use crate::storage::Storage;
use processing::{merge_statistics_with_scores, process_json_to_statistics};
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait EspnApiClient: Send + Sync {
    async fn get_json_from_espn(
        &self,
        scores: &[Scores],
        year: i32,
        event_id: i32,
    ) -> Result<PlayerJsonResponse, CoreError>;

    async fn fallback_scores(&self, _event_id: i32) -> Result<Option<Vec<Scores>>, CoreError> {
        Ok(None)
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait EspnApiClient {
    async fn get_json_from_espn(
        &self,
        scores: &[Scores],
        year: i32,
        event_id: i32,
    ) -> Result<PlayerJsonResponse, CoreError>;

    async fn fallback_scores(&self, _event_id: i32) -> Result<Option<Vec<Scores>>, CoreError> {
        Ok(None)
    }
}

pub async fn go_get_espn_data(
    api: &dyn EspnApiClient,
    scores: Vec<Scores>,
    year: i32,
    event_id: i32,
) -> Result<Vec<Scores>, CoreError> {
    let json_responses = api.get_json_from_espn(&scores, year, event_id).await?;
    let statistics = process_json_to_statistics(&json_responses)?;
    merge_statistics_with_scores(&statistics, &scores)
}

pub async fn fetch_scores_from_espn(
    api: &dyn EspnApiClient,
    storage: &dyn Storage,
    scores: Vec<Scores>,
    year: i32,
    event_id: i32,
    use_cache: bool,
    cache_max_age: i64,
) -> Result<ScoresAndLastRefresh, CoreError> {
    if use_cache {
        if let Some(cached) = crate::score::context::load_cached_scores(
            storage,
            event_id,
            cache_max_age,
        )
        .await?
        {
            return Ok(cached);
        }
    }

    let fetched_scores = match go_get_espn_data(api, scores, year, event_id).await {
        Ok(fetched) => fetched,
        Err(err) => {
            if let Ok(cached) = storage.get_scores(event_id, RefreshSource::Db).await {
                return Ok(cached);
            }
            if let Some(fallback) = api.fallback_scores(event_id).await? {
                fallback
            } else {
                return Err(err);
            }
        }
    };

    let existing_scores = match storage.get_scores(event_id, RefreshSource::Db).await {
        Ok(existing) => Some(existing.score_struct),
        Err(_) => None,
    };
    let merged_scores = merge_scores_for_event(&scores, fetched_scores, existing_scores);
    crate::score::context::store_scores_and_reload(storage, event_id, &merged_scores).await
}

fn merge_scores_for_event(
    expected_scores: &[Scores],
    fetched_scores: Vec<Scores>,
    existing_scores: Option<Vec<Scores>>,
) -> Vec<Scores> {
    let fetched_by_id: HashMap<i64, Scores> =
        fetched_scores.into_iter().map(|score| (score.eup_id, score)).collect();
    let existing_by_id: HashMap<i64, Scores> = existing_scores
        .unwrap_or_default()
        .into_iter()
        .map(|score| (score.eup_id, score))
        .collect();

    expected_scores
        .iter()
        .map(|expected| {
            fetched_by_id
                .get(&expected.eup_id)
                .cloned()
                .or_else(|| existing_by_id.get(&expected.eup_id).cloned())
                .unwrap_or_else(|| expected.clone())
        })
        .collect()
}
