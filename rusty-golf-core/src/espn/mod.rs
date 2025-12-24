pub mod processing;

use async_trait::async_trait;
use crate::error::CoreError;
use crate::model::{PlayerJsonResponse, Scores, ScoresAndLastRefresh};
use crate::storage::Storage;
use processing::{merge_statistics_with_scores, process_json_to_statistics};

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
            if let Some(fallback) = api.fallback_scores(event_id).await? {
                fallback
            } else {
                return Err(err);
            }
        }
    };

    crate::score::context::store_scores_and_reload(storage, event_id, &fetched_scores).await
}
