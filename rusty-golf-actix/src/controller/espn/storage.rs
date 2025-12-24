use crate::model::{RefreshSource, Scores, ScoresAndLastRefresh};
use rusty_golf_core::storage::Storage;

/// # Errors
///
/// Will return `Err` if the database query fails
pub async fn store_espn_results(
    scores: &[Scores],
    event_id: i32,
    storage: &dyn Storage,
) -> Result<ScoresAndLastRefresh, Box<dyn std::error::Error>> {
    storage.store_scores(event_id, scores).await?;
    Ok(storage.get_scores(event_id, RefreshSource::Espn).await?)
}
