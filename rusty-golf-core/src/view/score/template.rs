use maud::Markup;
use std::collections::HashMap;
use std::hash::BuildHasher;

use crate::model::ScoreData;
use crate::score::{group_by_bettor_golfer_round, group_by_bettor_name_and_round};
use crate::view::score::types::RefreshData;
use crate::view::score::{
    render_drop_down_bar_pure, render_line_score_tables, render_scoreboard, render_summary_scores,
};

#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn render_scores_template_pure<S: BuildHasher>(
    data: &ScoreData,
    expanded: bool,
    bettor_struct_for_line_scores: &[crate::view::score::types::BettorData],
    global_step_factor: f32,
    player_step_factors: &HashMap<(i64, String), f32, S>,
    event_id: i32,
    year: i32,
    _cache: bool,
) -> Markup {
    let summary_scores_x = group_by_bettor_name_and_round(&data.score_struct);
    let detailed_scores = group_by_bettor_golfer_round(&data.score_struct);

    let refresh_data = RefreshData {
        last_refresh: data.last_refresh.clone(),
        last_refresh_source: data.last_refresh_source.clone(),
    };

    // Fragments should always read from the warmed DB snapshot on initial page load
    // to avoid duplicate concurrent fetches; force cache=1 for hx requests.
    let cache_str = "1";

    maud::html! {
        (render_scoreboard(data))
        @if expanded {
            div id="score-summary"
                hx-get=(format!("scores/summary?event={}&yr={}&cache={}&expanded={}", event_id, year, cache_str, "1"))
                hx-trigger="load" hx-swap="innerHTML" {
                (render_summary_scores(&summary_scores_x))
            }
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
