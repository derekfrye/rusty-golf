use crate::error::CoreError;
use crate::espn::{EspnApiClient, fetch_scores_from_espn};
use crate::model::format_time_ago_for_score_view;
use crate::model::{Bettors, RefreshSource, ScoreData, Scores, ScoresAndLastRefresh};
use crate::storage::Storage;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ScoreContext {
    pub data: ScoreData,
    pub from_db_scores: ScoresAndLastRefresh,
    pub global_step_factor: f32,
    pub player_step_factors: HashMap<(i64, String), f32>,
}

#[must_use]
pub fn score_data_from_scores(scores: &ScoresAndLastRefresh) -> ScoreData {
    let cache_hit = matches!(scores.last_refresh_source, RefreshSource::Db);
    score_data_from_scores_with_cache(scores, cache_hit)
}

#[must_use]
pub fn score_data_from_scores_with_cache(
    scores: &ScoresAndLastRefresh,
    cache_hit: bool,
) -> ScoreData {
    let mut totals: HashMap<String, i32> = HashMap::new();
    for golfer in &scores.score_struct {
        *totals.entry(golfer.bettor_name.clone()).or_insert(0) +=
            golfer.detailed_statistics.total_score;
    }

    let mut bettors: Vec<Bettors> = totals
        .into_iter()
        .map(|(name, total)| Bettors {
            bettor_name: name,
            total_score: total,
            scoreboard_position_name: String::new(),
            scoreboard_position: 0,
        })
        .collect();

    bettors.sort_by(|a, b| {
        a.total_score
            .cmp(&b.total_score)
            .then_with(|| a.bettor_name.cmp(&b.bettor_name))
    });

    for (i, bettor) in bettors.iter_mut().enumerate() {
        bettor.scoreboard_position = i;
        bettor.scoreboard_position_name = match i {
            0 => "TOP GOLFER".to_string(),
            1 => "FIRST LOSER".to_string(),
            2 => "MEH".to_string(),
            3 => "SEEN BETTER DAYS".to_string(),
            4 => "NOT A CHANCE".to_string(),
            _ => "WORST OF THE WORST".to_string(),
        };
    }

    let elapsed = chrono::Utc::now().naive_utc() - scores.last_refresh;
    ScoreData {
        bettor_struct: bettors,
        score_struct: scores.score_struct.clone(),
        last_refresh: format_time_ago_for_score_view(elapsed),
        last_refresh_source: scores.last_refresh_source.clone(),
        cache_hit,
    }
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
    let active_golfers = storage.get_golfers_for_event(event_id).await?;
    let (scores_and_refresh, cache_hit) = fetch_scores_from_espn(
        espn_api,
        storage,
        active_golfers,
        year,
        event_id,
        use_cache,
        cache_max_age,
    )
    .await?;
    Ok(score_data_from_scores_with_cache(
        &scores_and_refresh,
        cache_hit,
    ))
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
    let active_golfers = storage.get_golfers_for_event(event_id).await?;
    let (scores_and_refresh, cache_hit) = fetch_scores_from_espn(
        espn_api,
        storage,
        active_golfers,
        year,
        event_id,
        use_cache,
        cache_max_age,
    )
    .await?;
    let data = score_data_from_scores_with_cache(&scores_and_refresh, cache_hit);
    let event_details = storage.get_event_details(event_id).await?;
    let player_step_factors = storage.get_player_step_factors(event_id).await?;
    Ok(ScoreContext {
        data,
        from_db_scores: scores_and_refresh,
        global_step_factor: event_details.score_view_step_factor,
        player_step_factors,
    })
}

/// Load cached scores if they are still valid.
///
/// # Errors
/// Returns `Ok(None)` when cache is missing, invalid, or unreadable.
pub async fn load_cached_scores(
    storage: &dyn Storage,
    event_id: i32,
    max_age_seconds: i64,
) -> Result<Option<ScoresAndLastRefresh>, CoreError> {
    if storage
        .event_and_scores_already_in_db(event_id, max_age_seconds)
        .await
        .unwrap_or(false)
    {
        match storage.get_scores(event_id, RefreshSource::Db).await {
            Ok(scores) => Ok(Some(scores)),
            Err(_) => Ok(None),
        }
    } else {
        Ok(None)
    }
}

/// Store scores and reload them from storage.
///
/// # Errors
/// Returns an error if the store or reload fails.
pub async fn store_scores_and_reload(
    storage: &dyn Storage,
    event_id: i32,
    scores: &[Scores],
) -> Result<ScoresAndLastRefresh, CoreError> {
    storage.store_scores(event_id, scores).await?;
    Ok(storage.get_scores(event_id, RefreshSource::Espn).await?)
}
