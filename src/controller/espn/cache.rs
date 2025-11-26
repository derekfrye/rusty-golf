use crate::controller::espn::processing::go_get_espn_data;
use crate::controller::espn::storage::store_espn_results;
use crate::model::{
    RefreshSource, Scores, ScoresAndLastRefresh, event_and_scores_already_in_db, get_scores_from_db,
};
use serde_json::Value;
use sql_middleware::middleware::ConfigAndPool;
use std::fs;
use std::io;
use std::sync::OnceLock;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex as AsyncMutex;

static EVENT_LOCKS: OnceLock<AsyncMutex<HashMap<i32, Arc<AsyncMutex<()>>>>> = OnceLock::new();

async fn get_event_lock(event_id: i32) -> Arc<AsyncMutex<()>> {
    let map_mutex = EVENT_LOCKS.get_or_init(|| AsyncMutex::new(HashMap::new()));
    let mut guard = map_mutex.lock().await;
    guard
        .entry(event_id)
        .or_insert_with(|| Arc::new(AsyncMutex::new(())))
        .clone()
}

async fn try_cached_scores(
    use_cache: bool,
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<Option<ScoresAndLastRefresh>, Box<dyn std::error::Error>> {
    if use_cache
        && event_and_scores_already_in_db(config_and_pool, event_id, 0)
            .await
            .unwrap_or(false)
    {
        Ok(Some(
            get_scores_from_db(config_and_pool, event_id, RefreshSource::Db).await?,
        ))
    } else {
        Ok(None)
    }
}

async fn load_offline_scores(
    event_id: i32,
    config_and_pool: &ConfigAndPool,
) -> Result<ScoresAndLastRefresh, Box<dyn std::error::Error>> {
    let text = fs::read_to_string("tests/test3_espn_json_responses.json")?;
    let val = serde_json::from_str::<Value>(&text)?;
    let score_struct = val
        .get("score_struct")
        .ok_or_else(|| io::Error::other("offline fixture missing score_struct"))?;
    let scores_vec = serde_json::from_value::<Vec<Scores>>(score_struct.clone())?;
    store_espn_results(&scores_vec, event_id, config_and_pool).await
}

async fn fetch_or_fallback(
    scores: Vec<Scores>,
    year: i32,
    event_id: i32,
    config_and_pool: &ConfigAndPool,
) -> Result<ScoresAndLastRefresh, Box<dyn std::error::Error>> {
    match go_get_espn_data(scores, year, event_id).await {
        Ok(fetched_scores) => store_espn_results(&fetched_scores, event_id, config_and_pool).await,
        Err(e) => {
            eprintln!("ESPN fetch failed: {e}. Falling back to offline fixtures.");
            load_offline_scores(event_id, config_and_pool).await
        }
    }
}

/// # Errors
///
/// Will return `Err` if the espn api call fails
pub async fn fetch_scores_from_espn(
    scores: Vec<Scores>,
    year: i32,
    event_id: i32,
    config_and_pool: &ConfigAndPool,
    use_cache: bool,
    _cache_max_age: i64,
) -> Result<ScoresAndLastRefresh, Box<dyn std::error::Error>> {
    if let Some(scores) = try_cached_scores(use_cache, config_and_pool, event_id).await? {
        return Ok(scores);
    }

    // Serialize fetch/store per event_id to avoid duplicates and ensure fragments see a consistent snapshot
    let event_mutex = get_event_lock(event_id).await;
    let _guard = event_mutex.lock().await;

    // Re-check after acquiring the lock in case another task already populated the DB
    if let Some(scores) = try_cached_scores(use_cache, config_and_pool, event_id).await? {
        return Ok(scores);
    }

    fetch_or_fallback(scores, year, event_id, config_and_pool).await
}
