use sql_middleware::middleware::ConfigAndPool;
use crate::model::{
    RefreshSource, Scores, ScoresAndLastRefresh, 
    get_scores_from_db, store_scores_in_db
};

pub async fn store_espn_results(
    scores: &[Scores],
    event_id: i32,
    config_and_pool: &ConfigAndPool,
) -> Result<ScoresAndLastRefresh, Box<dyn std::error::Error>> {
    store_scores_in_db(config_and_pool, event_id, scores).await?;
    Ok(get_scores_from_db(config_and_pool, event_id, RefreshSource::Espn).await?)
}