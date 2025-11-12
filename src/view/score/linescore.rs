use crate::model::take_a_char_off;
use crate::view::score::types::{BettorData, RefreshData};
use crate::view::score::utils::{score_with_shape, short_golfer_name};
use maud::{Markup, html};

#[must_use]
pub fn render_line_score_tables(bettors: &[BettorData], refresh_data: &RefreshData) -> Markup {
    html! {
        h3 class="playerbars" { "Score by Golfer" }

        p class="refresh" {
            "Use filters from prior section to cycle through golfers, and use buttons below to cycle through rounds."
        }

        @for (idx, bettor) in bettors.iter().enumerate() {
            @let visibility_class = if idx == 0 { "linescore-container visible" }
                                    else { "linescore-container hidden" };

            div class=(visibility_class) data-player=(bettor.bettor_name) {
                @for golfer in &bettor.golfers {
                    @let unique_rounds = (1..=golfer.tee_times.len()).collect::<Vec<_>>();

                    table class="linescore-table" {
                        thead {
                            tr {
                                th class="topheader" {
                                    (short_golfer_name(&golfer.golfer_name))
                                }
                                th colspan="2" class="topheader" {
                                    @for rd in unique_rounds.iter().take(2) {
                                        @let is_latest_round = rd == unique_rounds.last().unwrap_or(&0);
                                        @let row_class = if is_latest_round { "linescore-round-button selected" } else { "linescore-round-button" };

                                        button class=(row_class) data-round=(rd) {
                                            "R" (rd)
                                        }
                                        " "
                                    }
                                }
                            }
                            tr {
                                th class="topheader"  {
                                    @for rd in unique_rounds.iter() {
                                        @let is_latest_round = rd == unique_rounds.last().unwrap_or(&0);
                                        @let row_class = if is_latest_round { "topheader" } else { "topheader hidden" };

                                        @if golfer.tee_times.len() >= (*rd - 1) {
                                            @let a = &golfer.tee_times[*rd - 1].val;
                                            @let b = if a.ends_with("am") || a.ends_with("pm") {
                                                take_a_char_off(a)
                                            } else {
                                                a.to_string()
                                            };
                                            div class=(row_class) data-round=(rd) { "Tee (ct): " br { (b) }}
                                        }
                                    }
                                }
                                th colspan="2" class="topheader" {
                                    @for rd in unique_rounds.iter().skip(2) {
                                        @let is_latest_round = rd == unique_rounds.last().unwrap_or(&0);
                                        @let row_class = if is_latest_round { "linescore-round-button selected" } else { "linescore-round-button" };

                                        button class=(row_class) data-round=(rd) {
                                            "R" (rd)
                                        }
                                        " "
                                    }
                                }
                            }
                            tr {
                                th { "Hole" }
                                th { "Par" }
                                th { "Strokes" }
                            }
                        }
                        tbody {
                            @let all_scores = {
                                let mut scores = golfer.linescores.clone();
                                scores.sort_by_key(|ls| (ls.round, ls.hole));
                                scores
                            };

                            @for ls in all_scores.iter() {
                                @let is_latest_round = ls.round + 1 == i32::try_from(*unique_rounds.last().unwrap_or(&0)).unwrap_or(0);
                                @let row_class = if is_latest_round { "linescore-row" } else { "linescore-row hidden" };

                                tr class=(row_class) data-round=(ls.round + 1) {
                                    td { (ls.hole) }
                                    td { (ls.par) }
                                    td { (score_with_shape(&ls.score, &ls.score_display)) }
                                }
                            }

                            // Relative-to-par totals by round with legacy label/copy
                            @let totals_by_round = {
                                let mut x = std::collections::BTreeMap::new();
                                for ls in all_scores.iter() {
                                    x.entry(ls.round)
                                        .and_modify(|t| *t += ls.score - ls.par)
                                        .or_insert(ls.score - ls.par);
                                }
                                x
                            };

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
                }
            }
        }

        p class="refresh" {
            "Last refreshed from " (refresh_data.last_refresh_source) " " (refresh_data.last_refresh) " ago."
        }
    }
}
