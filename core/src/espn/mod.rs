pub mod processing;

use crate::error::CoreError;
use crate::model::{PlayerJsonResponse, RefreshSource, Scores, ScoresAndLastRefresh};
use crate::storage::Storage;
use crate::timed;
use crate::timing::TimingSink;
use async_trait::async_trait;
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

enum FetchOutcome {
    Scores(Vec<Scores>),
    Cached(ScoresAndLastRefresh),
}

pub struct FetchScoresRequest<'a> {
    pub api: &'a dyn EspnApiClient,
    pub storage: &'a dyn Storage,
    pub scores: Vec<Scores>,
    pub year: i32,
    pub event_id: i32,
    pub use_cache: bool,
    pub cache_max_age: i64,
    pub timing: Option<&'a dyn TimingSink>,
}

/// Fetch ESPN JSON and merge it into score records.
///
/// # Errors
/// Returns an error if the ESPN request fails or data cannot be processed.
pub async fn go_get_espn_data(
    api: &dyn EspnApiClient,
    scores: &[Scores],
    year: i32,
    event_id: i32,
) -> Result<Vec<Scores>, CoreError> {
    go_get_espn_data_with_timing(api, scores, year, event_id, None).await
}

/// Fetch ESPN JSON, process it, and merge it with score records, capturing timings.
///
/// # Errors
/// Returns an error if the ESPN request fails or data cannot be processed.
pub async fn go_get_espn_data_with_timing(
    api: &dyn EspnApiClient,
    scores: &[Scores],
    year: i32,
    event_id: i32,
    timing: Option<&dyn TimingSink>,
) -> Result<Vec<Scores>, CoreError> {
    let json_responses = timed!(
        timing,
        "espn.fetch_json_ms",
        api.get_json_from_espn(scores, year, event_id).await
    )?;
    let statistics = timed!(
        timing,
        "espn.process_json_ms",
        process_json_to_statistics(&json_responses)
    )?;
    timed!(
        timing,
        "espn.merge_statistics_ms",
        merge_statistics_with_scores(&statistics, scores)
    )
}

/// Fetch scores with optional caching and fallback logic.
///
/// # Errors
/// Returns an error if ESPN fetch, cache read/write, or fallback retrieval fails.
pub async fn fetch_scores_from_espn(
    api: &dyn EspnApiClient,
    storage: &dyn Storage,
    scores: Vec<Scores>,
    year: i32,
    event_id: i32,
    use_cache: bool,
    cache_max_age: i64,
) -> Result<(ScoresAndLastRefresh, bool), CoreError> {
    fetch_scores_from_espn_with_timing(FetchScoresRequest {
        api,
        storage,
        scores,
        year,
        event_id,
        use_cache,
        cache_max_age,
        timing: None,
    })
    .await
}

/// Fetch scores with optional caching and fallback logic, capturing timings.
///
/// # Errors
/// Returns an error if ESPN fetch, cache read/write, or fallback retrieval fails.
pub async fn fetch_scores_from_espn_with_timing(
    request: FetchScoresRequest<'_>,
) -> Result<(ScoresAndLastRefresh, bool), CoreError> {
    let FetchScoresRequest {
        api,
        storage,
        scores,
        year,
        event_id,
        use_cache,
        cache_max_age,
        timing,
    } = request;

    if use_cache && cache_max_age < 0 {
        let cached = timed!(
            timing,
            "cache.get_scores_db_ms",
            storage.get_scores(event_id, RefreshSource::Db).await
        );
        if let Ok(cached) = cached {
            return Ok((cached, true));
        }
    }

    if use_cache {
        let cached = timed!(
            timing,
            "cache.load_cached_scores_ms",
            crate::score::context::load_cached_scores(
                storage,
                event_id,
                cache_max_age,
                timing,
            )
            .await
        )?;
        if let Some(cached) = cached {
            return Ok((cached, true));
        }
    }

    let fetch_outcome = timed!(
        timing,
        "espn.fetch_total_ms",
        async {
            match go_get_espn_data_with_timing(api, &scores, year, event_id, timing).await {
                Ok(fetched) => Ok(FetchOutcome::Scores(fetched)),
                Err(err) => {
                    if let Ok(cached) = storage.get_scores(event_id, RefreshSource::Db).await {
                        Ok(FetchOutcome::Cached(cached))
                    } else {
                        match api.fallback_scores(event_id).await {
                            Ok(Some(fallback)) => Ok(FetchOutcome::Scores(fallback)),
                            Ok(None) => Err(err),
                            Err(fallback_err) => Err(fallback_err),
                        }
                    }
                }
            }
        }
        .await
    )?;

    let fetched_scores = match fetch_outcome {
        FetchOutcome::Scores(scores) => scores,
        FetchOutcome::Cached(cached) => return Ok((cached, false)),
    };

    let existing_scores = timed!(
        timing,
        "storage.get_scores_db_ms",
        storage.get_scores(event_id, RefreshSource::Db).await
    );
    let existing_scores = match existing_scores {
        Ok(existing) => Some(existing.score_struct),
        Err(_) => None,
    };
    let merged_scores = timed!(
        timing,
        "scores.merge_event_ms",
        merge_scores_for_event(&scores, fetched_scores, existing_scores)
    );
    let stored = timed!(
        timing,
        "storage.store_scores_ms",
        crate::score::context::store_scores_and_reload(storage, event_id, &merged_scores).await
    )?;
    Ok((stored, false))
}

fn merge_scores_for_event(
    expected_scores: &[Scores],
    fetched_scores: Vec<Scores>,
    existing_scores: Option<Vec<Scores>>,
) -> Vec<Scores> {
    let fetched_by_id: HashMap<i64, Scores> = fetched_scores
        .into_iter()
        .map(|score| (score.eup_id, score))
        .collect();
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
