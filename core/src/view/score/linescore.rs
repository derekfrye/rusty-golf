use crate::model::LineScore;
use crate::model::take_a_char_off;
use crate::view::score::types::{BettorData, GolferData, RefreshData};
use crate::view::score::utils::{score_with_shape, short_golfer_name};
use maud::{Markup, html};
use std::collections::{BTreeMap, BTreeSet};

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

fn select_round_for_golfer(rounds: &[usize], default_round: usize) -> usize {
    match rounds.last().copied() {
        Some(max_round) => default_round.min(max_round).max(1),
        None => 1,
    }
}

fn determine_default_round(bettors: &[BettorData]) -> usize {
    let golfers = bettors.iter().flat_map(|bettor| bettor.golfers.iter());
    let current_round = golfers
        .clone()
        .filter_map(highest_started_round)
        .max()
        .unwrap_or(1);

    let everyone_done_with_current_round = golfers
        .filter(|golfer| golfer_has_round_available(golfer, current_round))
        .all(|golfer| golfer_completed_round(golfer, current_round));

    if everyone_done_with_current_round {
        current_round + 1
    } else {
        current_round
    }
}

fn highest_started_round(golfer: &GolferData) -> Option<usize> {
    golfer
        .linescores
        .iter()
        .filter_map(|ls| usize::try_from(ls.round + 1).ok())
        .max()
}

fn golfer_has_round_available(golfer: &GolferData, round: usize) -> bool {
    golfer.tee_times.len() >= round
        || highest_started_round(golfer).is_some_and(|started| started >= round)
}

fn golfer_completed_round(golfer: &GolferData, round: usize) -> bool {
    let round_zero_based = match i32::try_from(round) {
        Ok(value) => value - 1,
        Err(_) => return false,
    };

    let completed_holes = golfer
        .linescores
        .iter()
        .filter(|ls| ls.round == round_zero_based)
        .map(|ls| ls.hole)
        .collect::<BTreeSet<_>>();

    completed_holes.len() >= 18
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ScoreDisplay, StringStat};
    use crate::view::score::types::BettorData;

    fn tee_times(count: usize) -> Vec<StringStat> {
        (1..=count)
            .map(|idx| StringStat {
                val: format!("4/{idx:02} 8:00a"),
            })
            .collect()
    }

    fn line_scores(round: i32, holes: std::ops::RangeInclusive<i32>) -> Vec<LineScore> {
        holes
            .map(|hole| LineScore {
                round,
                hole,
                score: 4,
                par: 4,
                score_display: ScoreDisplay::Par,
            })
            .collect()
    }

    fn golfer(name: &str, tee_time_count: usize, linescores: Vec<LineScore>) -> GolferData {
        GolferData {
            golfer_name: name.to_string(),
            linescores,
            tee_times: tee_times(tee_time_count),
        }
    }

    fn bettors(golfers: Vec<GolferData>) -> Vec<BettorData> {
        vec![BettorData {
            bettor_name: "Chris".to_string(),
            golfers,
        }]
    }

    #[test]
    fn keeps_current_round_selected_while_anyone_is_still_playing() {
        let bettors = bettors(vec![
            golfer("A", 3, line_scores(0, 1..=18)),
            golfer("B", 3, line_scores(0, 1..=7)),
        ]);

        assert_eq!(determine_default_round(&bettors), 1);
    }

    #[test]
    fn advances_only_one_round_after_everyone_finishes_current_round() {
        let bettors = bettors(vec![
            golfer("A", 3, line_scores(0, 1..=18)),
            golfer("B", 3, line_scores(0, 1..=18)),
        ]);

        assert_eq!(determine_default_round(&bettors), 2);
    }

    #[test]
    fn advances_to_round_three_only_after_round_two_is_complete() {
        let mut golfer_a_scores = line_scores(0, 1..=18);
        golfer_a_scores.extend(line_scores(1, 1..=18));
        let mut golfer_b_scores = line_scores(0, 1..=18);
        golfer_b_scores.extend(line_scores(1, 1..=18));

        let bettors = bettors(vec![
            golfer("A", 4, golfer_a_scores),
            golfer("B", 4, golfer_b_scores),
        ]);

        assert_eq!(determine_default_round(&bettors), 3);
    }

    #[test]
    fn initial_markup_uses_selected_round_for_totals_and_buttons() {
        let bettors = bettors(vec![golfer("A", 2, line_scores(0, 1..=6))]);
        let refresh = RefreshData {
            last_refresh: "1 minute".to_string(),
            last_refresh_source: crate::model::RefreshSource::Kv,
        };

        let markup = render_line_score_tables(&bettors, &refresh).into_string();

        assert!(markup.contains("linescore-round-button selected\" data-round=\"1\""));
        assert!(markup.contains("tr class=\"linescore-total\" data-round=\"1\""));
        assert!(markup.contains("tr class=\"linescore-row\" data-round=\"1\""));
        assert!(!markup.contains("linescore-total\" data-round=\"2\""));
    }
}
