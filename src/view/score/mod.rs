pub use rusty_golf_core::view::score::*;

use maud::Markup;
use rusty_golf_core::storage::Storage;
use sql_middleware::middleware::ConfigAndPool;

use crate::model::{RefreshSource, ScoreData};
use crate::storage::SqlStorage;

/// Backward-compatible async wrapper used by existing tests. Performs IO then renders purely.
///
/// # Errors
///
/// Returns an error if storage queries fail.
pub async fn render_scores_template(
    data: &ScoreData,
    expanded: bool,
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<Markup, Box<dyn std::error::Error>> {
    let storage = SqlStorage::new(config_and_pool.clone());
    let from_db_scores = storage.get_scores(event_id, RefreshSource::Db).await?;
    let bettor_struct = scores_and_last_refresh_to_line_score_tables(&from_db_scores);
    let event_details = storage.get_event_details(event_id).await?;
    let player_step_factors = storage.get_player_step_factors(event_id).await?;

    Ok(render_scores_template_pure(
        data,
        expanded,
        &bettor_struct,
        event_details.score_view_step_factor,
        &player_step_factors,
        event_id,
        0,
        true,
    ))
}
