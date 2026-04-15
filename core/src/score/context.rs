use crate::error::CoreError;
use crate::espn::{EspnApiClient, FetchScoresRequest, fetch_scores_from_espn_with_timing};
use crate::model::{ScoreData, ScoresAndLastRefresh};
use crate::storage::Storage;
use crate::timed;
use crate::timing::TimingSink;
use std::collections::HashMap;

mod context_cache;
mod context_data;

pub use context_cache::{load_cached_scores, store_scores_and_reload};
pub use context_data::{score_data_from_scores, score_data_from_scores_with_cache};

#[derive(Debug)]
pub struct ScoreContext {
    pub data: ScoreData,
    pub from_db_scores: ScoresAndLastRefresh,
    pub global_step_factor: f32,
    pub player_step_factors: HashMap<(i64, String), f32>,
}

/// Load scores and compute the score data view model.
///
/// # Errors
/// Returns an error if storage or ESPN fetch operations fail.
pub async fn load_scores_data(
    storage: &dyn Storage,
    espn_api: &dyn EspnApiClient,
    event_id: i32,
    year: i32,
    use_cache: bool,
    cache_max_age: i64,
) -> Result<ScoreData, CoreError> {
    load_scores_data_with_timing(
        storage,
        espn_api,
        event_id,
        year,
        use_cache,
        cache_max_age,
        None,
    )
    .await
}

/// Load scores and compute the score data view model, capturing timings.
///
/// # Errors
/// Returns an error if storage or ESPN fetch operations fail.
pub async fn load_scores_data_with_timing(
    storage: &dyn Storage,
    espn_api: &dyn EspnApiClient,
    event_id: i32,
    year: i32,
    use_cache: bool,
    cache_max_age: i64,
    timing: Option<&dyn TimingSink>,
) -> Result<ScoreData, CoreError> {
    let active_golfers = timed!(
        timing,
        "storage.get_golfers_for_event_ms",
        storage.get_golfers_for_event(event_id).await
    )?;
    let (scores_and_refresh, cache_hit) = timed!(
        timing,
        "score_context.fetch_scores_ms",
        fetch_scores_from_espn_with_timing(FetchScoresRequest {
            api: espn_api,
            storage,
            scores: active_golfers,
            year,
            event_id,
            use_cache,
            cache_max_age,
            timing,
        },)
        .await
    )?;
    let data = timed!(
        timing,
        "score_context.build_score_data_ms",
        score_data_from_scores_with_cache(&scores_and_refresh, cache_hit)
    );
    Ok(data)
}

/// Load full score context, including step factors.
///
/// # Errors
/// Returns an error if storage or ESPN fetch operations fail.
pub async fn load_score_context(
    storage: &dyn Storage,
    espn_api: &dyn EspnApiClient,
    event_id: i32,
    year: i32,
    use_cache: bool,
    cache_max_age: i64,
) -> Result<ScoreContext, CoreError> {
    load_score_context_with_timing(
        storage,
        espn_api,
        event_id,
        year,
        use_cache,
        cache_max_age,
        None,
    )
    .await
}

/// Load full score context, including step factors, capturing timings.
///
/// # Errors
/// Returns an error if storage or ESPN fetch operations fail.
pub async fn load_score_context_with_timing(
    storage: &dyn Storage,
    espn_api: &dyn EspnApiClient,
    event_id: i32,
    year: i32,
    use_cache: bool,
    cache_max_age: i64,
    timing: Option<&dyn TimingSink>,
) -> Result<ScoreContext, CoreError> {
    let active_golfers = timed!(
        timing,
        "storage.get_golfers_for_event_ms",
        storage.get_golfers_for_event(event_id).await
    )?;
    let (scores_and_refresh, cache_hit) = timed!(
        timing,
        "score_context.fetch_scores_ms",
        fetch_scores_from_espn_with_timing(FetchScoresRequest {
            api: espn_api,
            storage,
            scores: active_golfers,
            year,
            event_id,
            use_cache,
            cache_max_age,
            timing,
        },)
        .await
    )?;
    let data = timed!(
        timing,
        "score_context.build_score_data_ms",
        score_data_from_scores_with_cache(&scores_and_refresh, cache_hit)
    );
    let event_details = timed!(
        timing,
        "storage.get_event_details_ms",
        storage.get_event_details(event_id).await
    )?;
    let player_step_factors = timed!(
        timing,
        "storage.get_player_step_factors_ms",
        storage.get_player_step_factors(event_id).await
    )?;
    Ok(ScoreContext {
        data,
        from_db_scores: scores_and_refresh,
        global_step_factor: event_details.score_view_step_factor,
        player_step_factors,
    })
}
