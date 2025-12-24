use crate::controller::espn::ActixEspnClient;
use rusty_golf_core::error::CoreError;
use rusty_golf_core::score::load_scores_data;
use rusty_golf_core::storage::Storage;

/// # Errors
///
/// Will return `Err` if the database or espn api call fails
pub async fn get_data_for_scores_page(
    event_id: i32,
    year: i32,
    use_cache: bool,
    storage: &dyn Storage,
    cache_max_age: i64,
) -> Result<rusty_golf_core::model::ScoreData, CoreError> {
    let espn_client = ActixEspnClient::new();
    load_scores_data(
        storage,
        &espn_client,
        event_id,
        year,
        use_cache,
        cache_max_age,
    )
    .await
}
