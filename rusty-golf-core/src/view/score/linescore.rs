use crate::model::take_a_char_off;
use crate::view::score::types::{BettorData, GolferData, RefreshData};
use crate::view::score::utils::{score_with_shape, short_golfer_name};
use maud::{Markup, html};

#[must_use]
pub fn render_line_score_tables(bettors: &[BettorData], refresh_data: &RefreshData) -> Markup {
    html! {
        h3 class="playerbars" { "Score by Golfer" }

        p class="refresh" {
            "Use filters from prior section to cycle through golfers, and use buttons below to cycle through rounds."
        }

        (render_bettor_sections(bettors))

        p class="refresh" {
            "Last refreshed from " (refresh_data.last_refresh_source) " " (refresh_data.last_refresh) " ago."
        }
    }
}

fn render_bettor_sections(bettors: &[BettorData]) -> Markup {
    html! {
        @for (idx, bettor) in bettors.iter().enumerate() {
            (render_bettor_section(bettor, idx == 0))
        }
    }
}

fn render_bettor_section(bettor: &BettorData, is_visible: bool) -> Markup {
    let visibility_class = if is_visible {
        "linescore-container visible"
    } else {
        "linescore-container hidden"
    };

    html! {
        div class=(visibility_class) data-player=(bettor.bettor_name) {
            @for golfer in &bettor.golfers {
                (render_golfer_table(golfer))
            }
        }
    }
}

fn render_golfer_table(golfer: &GolferData) -> Markup {
    let unique_rounds = build_round_list(golfer);
    html! {
        table class="linescore-table" {
            thead { (render_table_header(golfer, &unique_rounds)) }
            tbody { (render_table_body(golfer, &unique_rounds)) }
        }
    }
}

fn build_round_list(golfer: &GolferData) -> Vec<usize> {
    (1..=golfer.tee_times.len()).collect::<Vec<_>>()
}

fn render_table_header(golfer: &GolferData, rounds: &[usize]) -> Markup {
    let latest_round = rounds.last().copied().unwrap_or_default();
    let first_buttons = rounds.iter().copied().take(2).collect::<Vec<_>>();
    let remaining_buttons = rounds.iter().copied().skip(2).collect::<Vec<_>>();

    html! {
        tr {
            th class="topheader" {
                (short_golfer_name(&golfer.golfer_name))
            }
            th colspan="2" class="topheader" {
                (render_round_buttons(&first_buttons, latest_round))
            }
        }
        tr {
            th class="topheader"  {
                (render_tee_time_rows(golfer, rounds, latest_round))
            }
            th colspan="2" class="topheader" {
                (render_round_buttons(&remaining_buttons, latest_round))
            }
        }
        tr {
            th { "Hole" }
            th { "Par" }
            th { "Strokes" }
        }
    }
}

fn render_round_buttons(rounds: &[usize], latest_round: usize) -> Markup {
    html! {
        @for rd in rounds {
            @let is_latest_round = *rd == latest_round;
            @let row_class = if is_latest_round { "linescore-round-button selected" } else { "linescore-round-button" };

            button class=(row_class) data-round=(rd) { "R" (rd) }
            " "
        }
    }
}

fn render_tee_time_rows(golfer: &GolferData, rounds: &[usize], latest_round: usize) -> Markup {
    html! {
        @for rd in rounds {
            @let is_latest_round = *rd == latest_round;
            @let row_class = if is_latest_round { "topheader" } else { "topheader hidden" };

            @if golfer.tee_times.len() >= (*rd - 1) {
                @let tee_time = &golfer.tee_times[*rd - 1].val;
                @let friendly_time = if tee_time.ends_with("am") || tee_time.ends_with("pm") {
                    take_a_char_off(tee_time)
                } else {
                    tee_time.clone()
                };
                div class=(row_class) data-round=(rd) { "Tee (ct): " br { (friendly_time) }}
            }
        }
    }
}

fn render_table_body(golfer: &GolferData, rounds: &[usize]) -> Markup {
    let mut all_scores = golfer.linescores.clone();
    all_scores.sort_by_key(|ls| (ls.round, ls.hole));
    let totals_by_round = build_totals(&all_scores);
    let latest_round = i32::try_from(*rounds.last().unwrap_or(&0)).unwrap_or(0);

    html! {
        @for ls in all_scores.iter() {
            @let is_latest_round = ls.round + 1 == latest_round;
            @let row_class = if is_latest_round { "linescore-row" } else { "linescore-row hidden" };

            tr class=(row_class) data-round=(ls.round + 1) {
                td { (ls.hole) }
                td { (ls.par) }
                td { (score_with_shape(&ls.score, &ls.score_display)) }
            }
        }

        @for (round_zero_based, total_rel) in totals_by_round.iter() {
            @let is_round_one = *round_zero_based == 0;
            @let row_class = if is_round_one { "linescore-total" } else { "linescore-total hidden" };

            tr class=(row_class) data-round=(round_zero_based + 1) {
                td data-round=(round_zero_based + 1) colspan="2" class="linescore-total-cell" {
                    "Total:"
                }
                td { (total_rel) }
            }
        }
    }
}

fn build_totals(all_scores: &[crate::model::LineScore]) -> std::collections::BTreeMap<i32, i32> {
    let mut totals = std::collections::BTreeMap::new();
    for ls in all_scores {
        totals
            .entry(ls.round)
            .and_modify(|t| *t += ls.score - ls.par)
            .or_insert(ls.score - ls.par);
    }
    totals
}
