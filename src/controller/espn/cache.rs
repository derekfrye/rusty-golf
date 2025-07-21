use crate::controller::espn::processing::go_get_espn_data;
use crate::controller::espn::storage::store_espn_results;
use crate::model::{
    RefreshSource, Scores, ScoresAndLastRefresh, event_and_scores_already_in_db, get_scores_from_db,
};
use sql_middleware::middleware::ConfigAndPool;

/// # Errors
///
/// Will return `Err` if the espn api call fails
pub async fn fetch_scores_from_espn(
    scores: Vec<Scores>,
    year: i32,
    event_id: i32,
    config_and_pool: &ConfigAndPool,
    use_cache: bool,
    cache_max_age: i64,
) -> Result<ScoresAndLastRefresh, Box<dyn std::error::Error>> {
    let are_we_using_cache: bool = if use_cache {
        event_and_scores_already_in_db(config_and_pool, event_id, cache_max_age)
            .await
            .unwrap_or(false)
    } else {
        false
    };

    if are_we_using_cache {
        Ok(get_scores_from_db(config_and_pool, event_id, RefreshSource::Db).await?)
    } else {
        let x = go_get_espn_data(scores, year, event_id).await?;
        let z = store_espn_results(&x, event_id, config_and_pool).await?;
        Ok(z)
    }
}
