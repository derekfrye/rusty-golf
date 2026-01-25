use crate::error::CoreError;
use crate::model::{RefreshSource, Scores, ScoresAndLastRefresh};
use crate::storage::Storage;
use crate::timed;
use crate::timing::TimingSink;

/// Load cached scores if they are still valid.
///
/// # Errors
/// Returns `Ok(None)` when cache is missing, invalid, or unreadable.
pub async fn load_cached_scores(
    storage: &dyn Storage,
    event_id: i32,
    max_age_seconds: i64,
    timing: Option<&dyn TimingSink>,
) -> Result<Option<ScoresAndLastRefresh>, CoreError> {
    let has_fresh_scores = timed!(
        timing,
        "cache.event_and_scores_fresh_ms",
        storage
            .event_and_scores_already_in_db(event_id, max_age_seconds)
            .await
            .unwrap_or(false)
    );
    if has_fresh_scores {
        match timed!(
            timing,
            "cache.get_scores_ms",
            storage.get_scores(event_id, RefreshSource::Db).await
        ) {
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
