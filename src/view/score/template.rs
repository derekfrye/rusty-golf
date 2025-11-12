use maud::Markup;
use std::collections::HashMap;
use sql_middleware::middleware::ConfigAndPool;

use crate::model::{ScoreData, RefreshSource};
use crate::model::event::get_event_details;
use crate::model::golfer::get_player_step_factors;
use crate::model::database_read::get_scores_from_db;
use crate::view::score::types::RefreshData;
use crate::view::score::{
    render_drop_down_bar_pure, render_line_score_tables, render_scoreboard, render_summary_scores,
    scores_and_last_refresh_to_line_score_tables,
};

#[must_use]
pub fn render_scores_template_pure(
    data: &ScoreData,
    expanded: bool,
    bettor_struct_for_line_scores: &[crate::view::score::types::BettorData],
    global_step_factor: f32,
    player_step_factors: &HashMap<(i64, String), f32>,
    event_id: i32,
    year: i32,
    cache: bool,
) -> Markup {
    let summary_scores_x =
        crate::controller::score::group_by_bettor_name_and_round(&data.score_struct);
    let detailed_scores =
        crate::controller::score::group_by_bettor_golfer_round(&data.score_struct);

    let refresh_data = RefreshData {
        last_refresh: data.last_refresh.clone(),
        last_refresh_source: data.last_refresh_source.clone(),
    };

    let cache_str = if cache { "1" } else { "0" };

    maud::html! {
        (render_scoreboard(data))
        div id="score-summary"
            hx-get=(format!("scores/summary?event={}&yr={}&cache={}&expanded={}", event_id, year, cache_str, if expanded {"1"} else {"0"}))
            hx-trigger="load" hx-swap="innerHTML" {
            @if expanded { (render_summary_scores(&summary_scores_x)) }
        }

        div id="score-chart"
            hx-get=(format!("scores/chart?event={}&yr={}&cache={}&expanded={}", event_id, year, cache_str, if expanded {"1"} else {"0"}))
            hx-trigger="load" hx-swap="innerHTML" {
            (render_drop_down_bar_pure(&summary_scores_x, &detailed_scores, global_step_factor, player_step_factors))
        }

        div id="linescore"
            hx-get=(format!("scores/linescore?event={}&yr={}&cache={}&expanded={}", event_id, year, cache_str, if expanded {"1"} else {"0"}))
            hx-trigger="load" hx-swap="innerHTML" {
            (render_line_score_tables(bettor_struct_for_line_scores, &refresh_data))
        }
    }
}

/// Backward-compatible async wrapper used by existing tests. Performs IO then renders purely.
///
/// # Errors
///
/// Returns an error if DB queries fail.
pub async fn render_scores_template(
    data: &ScoreData,
    expanded: bool,
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<Markup, Box<dyn std::error::Error>> {
    let from_db_scores = get_scores_from_db(config_and_pool, event_id, RefreshSource::Db).await?;
    let bettor_struct = scores_and_last_refresh_to_line_score_tables(&from_db_scores);
    let event_details = get_event_details(config_and_pool, event_id).await?;
    let player_step_factors = get_player_step_factors(config_and_pool, event_id).await?;

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
