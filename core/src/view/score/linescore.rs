use crate::model::LineScore;
use crate::model::take_a_char_off;
use crate::view::score::types::{BettorData, GolferData, RefreshData};
use crate::view::score::utils::{score_with_shape, short_golfer_name};
use maud::{Markup, html};
use std::collections::BTreeMap;

mod round_selection;
#[cfg(test)]
mod tests;

use round_selection::determine_default_round;

#[must_use]
pub fn render_line_score_tables(bettors: &[BettorData], refresh_data: &RefreshData) -> Markup {
    let default_round = determine_default_round(bettors);
    html! {
        h2 class="playerbars" { "Score by Golfer" }

        p class="refresh" {
            "Use filters from prior section to cycle through golfers, and use buttons below to cycle through rounds."
        }

        (render_bettor_sections(bettors, default_round))

        p class="refresh" {
            "Last refreshed from " (refresh_data.last_refresh_source) " " (refresh_data.last_refresh) " ago."
        }
    }
}

fn render_bettor_sections(bettors: &[BettorData], default_round: usize) -> Markup {
    html! {
        @for (idx, bettor) in bettors.iter().enumerate() {
            (render_bettor_section(bettor, idx == 0, default_round))
        }
    }
}

fn render_bettor_section(bettor: &BettorData, is_visible: bool, default_round: usize) -> Markup {
    let visibility_class = if is_visible {
        "linescore-container visible"
    } else {
        "linescore-container hidden"
    };

    html! {
        div class=(visibility_class) data-player=(bettor.bettor_name) {
            @for golfer in &bettor.golfers {
                (render_golfer_table(golfer, default_round))
            }
        }
    }
}

fn render_golfer_table(golfer: &GolferData, default_round: usize) -> Markup {
    let unique_rounds = build_round_list(golfer);
    let selected_round = select_round_for_golfer(&unique_rounds, default_round);
    html! {
        table class="linescore-table" {
            thead { (render_table_header(golfer, &unique_rounds, selected_round)) }
            tbody { (render_table_body(golfer, selected_round)) }
        }
    }
}

fn build_round_list(golfer: &GolferData) -> Vec<usize> {
    let highest_scored_round = golfer
        .linescores
        .iter()
        .filter_map(|ls| usize::try_from(ls.round + 1).ok())
        .max()
        .unwrap_or(0);
    let highest_round = golfer.tee_times.len().max(highest_scored_round);
    (1..=highest_round).collect::<Vec<_>>()
}

fn render_table_header(golfer: &GolferData, rounds: &[usize], selected_round: usize) -> Markup {
    let first_buttons = rounds.iter().copied().take(2).collect::<Vec<_>>();
    let remaining_buttons = rounds.iter().copied().skip(2).collect::<Vec<_>>();

    html! {
        tr {
            th class="topheader" {
                (short_golfer_name(&golfer.golfer_name))
            }
            th colspan="2" class="topheader" {
                (render_round_buttons(&first_buttons, selected_round))
            }
        }
        tr {
            th class="topheader"  {
                (render_tee_time_rows(golfer, rounds, selected_round))
            }
            th colspan="2" class="topheader" {
                (render_round_buttons(&remaining_buttons, selected_round))
            }
        }
        tr {
            th { "Hole" }
            th { "Par" }
            th { "Strokes" }
        }
    }
}

fn render_round_buttons(rounds: &[usize], selected_round: usize) -> Markup {
    html! {
        @for rd in rounds {
            @let is_selected_round = *rd == selected_round;
            @let row_class = if is_selected_round { "linescore-round-button selected" } else { "linescore-round-button" };

            button class=(row_class) data-round=(rd) { "R" (rd) }
            " "
        }
    }
}

fn render_tee_time_rows(golfer: &GolferData, rounds: &[usize], selected_round: usize) -> Markup {
    html! {
        @for rd in rounds {
            @let is_selected_round = *rd == selected_round;
            @let row_class = if is_selected_round { "topheader" } else { "topheader hidden" };

            @if let Some(tee_time) = golfer.tee_times.get(*rd - 1).map(|tt| &tt.val) {
                @let friendly_time = if tee_time.ends_with("am") || tee_time.ends_with("pm") {
                    take_a_char_off(tee_time)
                } else {
                    tee_time.clone()
                };
                div class=(row_class) data-round=(rd) {
                    "Tee (ct): "
                    br;
                    (friendly_time)
                }
            }
        }
    }
}

fn render_table_body(golfer: &GolferData, selected_round: usize) -> Markup {
    let mut all_scores = golfer.linescores.clone();
    all_scores.sort_by_key(|ls| (ls.round, ls.hole));
    let totals_by_round = build_totals(&all_scores);
    let selected_round = i32::try_from(selected_round).unwrap_or(0);

    html! {
        @for ls in all_scores.iter() {
            @let is_selected_round = ls.round + 1 == selected_round;
            @let row_class = if is_selected_round { "linescore-row" } else { "linescore-row hidden" };

            tr class=(row_class) data-round=(ls.round + 1) {
                td { (ls.hole) }
                td { (ls.par) }
                td { (score_with_shape(&ls.score, &ls.score_display)) }
            }
        }

        @for (round_zero_based, total_rel) in totals_by_round.iter() {
            @let is_selected_round = *round_zero_based + 1 == selected_round;
            @let row_class = if is_selected_round { "linescore-total" } else { "linescore-total hidden" };

            tr class=(row_class) data-round=(round_zero_based + 1) {
                td data-round=(round_zero_based + 1) colspan="2" class="linescore-total-cell" {
                    "Total:"
                }
                td { (total_rel) }
            }
        }
    }
}

fn build_totals(all_scores: &[LineScore]) -> BTreeMap<i32, i32> {
    let mut totals = BTreeMap::new();
    for ls in all_scores {
        totals
            .entry(ls.round)
            .and_modify(|t| *t += ls.score - ls.par)
            .or_insert(ls.score - ls.par);
    }
    totals
}

pub(crate) fn select_round_for_golfer(rounds: &[usize], default_round: usize) -> usize {
    match rounds.last().copied() {
        Some(max_round) => default_round.min(max_round).max(1),
        None => 1,
    }
}
