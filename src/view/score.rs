use crate::controller::score::group_by_scores;
use crate::model::{AllBettorScoresByRound, ScoreData};

use maud::{html, Markup};

pub fn render_scores_template(data: &ScoreData, expanded: bool) -> Markup {
    let summary_scores_x =
        crate::controller::score::group_by_bettor_name_and_round(&data.score_struct);

    html! {
        @if !data.score_struct.is_empty() {
            (render_scoreboard(data))
            @if expanded {
                (render_summary_scores(&summary_scores_x))
            }
            // (render_stacked_bar_chart(data))
            (render_drop_down_bar(data, &summary_scores_x))
            (render_score_detail(data))
        }
    }
}

fn render_scoreboard(data: &ScoreData) -> Markup {
    html! {
        @if !data.score_struct.is_empty(){

            p class="refresh" {
                "Last refreshed from ESPN " (data.last_refresh) " ago."
            }

            @let grouped_bettors = &data.bettor_struct;

            h3 { "Scoreboard" }

            table class="styled-table" {
                thead {
                    tr {
                        th { "PLACE" }
                        th { "PLAYER" }
                        th { "SCORE" }
                    }
                }
                tbody {
                    @for bettor in grouped_bettors {
                        tr {
                            td { (bettor.scoreboard_position_name) }
                            td { (bettor.bettor_name) }
                            td { (bettor.total_score) }
                        }
                    }
                }
            }
        }
        @else {
            table id="scores-table-1" {
                thead {
                    tr {
                        th rowspan="2" { "Player" }
                        th rowspan="2" { "Pick" }
                        th colspan="3" { "Round 1" }
                        th colspan="3" { "Round 2" }
                        th colspan="3" { "Round 3" }
                        th colspan="3" { "Round 4" }
                        th rowspan="2" { "Total" }
                    }
                    tr {
                        th { "Tee Time" }
                        th { "Score" }
                        th { "Tee Time" }
                        th { "Score" }
                        th { "Tee Time" }
                        th { "Score" }
                        th { "Tee Time" }
                        th { "Score" }
                    }
                }
                tbody {
                    tr {
                        td colspan="15" { "No scores available." }
                    }
                }
            }
        }

    }
}

fn render_summary_scores(grouped_data: &AllBettorScoresByRound) -> Markup {
    html! {
        @let summary_scores = grouped_data;

        @if !summary_scores.summary_scores.is_empty() {
            h3 { "Summary" }

            table class="summary" {
                thead {
                    tr {
                        th { "Player" }
                        @let num_rounds = summary_scores.summary_scores[0].computed_rounds.len();
                        @for round in 0..num_rounds {
                            th { "R" (round + 1) }
                        }
                        th { "Tot" }
                    }
                }
                tbody {
                    @for summary_score in &summary_scores.summary_scores {
                        tr {
                            td { (summary_score.bettor_name) }
                            @for (idx, _round) in summary_score.computed_rounds.iter().enumerate() {
                                @let score = summary_score.scores_aggregated_by_golf_grp_by_rd[idx];
                                td { (score) }
                            }
                            @let total = summary_score.scores_aggregated_by_golf_grp_by_rd.iter().sum::<isize>();
                            td { (total) }
                        }
                    }
                }
            }
        }
    }
}

fn render_thead(max_len_of_tee_times_in_rounds: usize, group: &usize) -> Markup {
    html! {
        thead {
            tr {
                th class="topheader" rowspan="2" {
                    "Player"
                }
                th class="topheader" rowspan="2" {
                    "Pick"
                }

                @for round in 0..max_len_of_tee_times_in_rounds {
                    th class="topheader shrinkable" colspan="3" data-round=({ round + 1 }) {
                        span class="toggle" data-round=({ round + 1 }) onclick=(format!("toggleRound({})", round + 1)) {
                            "Round " (round + 1)
                        }
                        br;
                        span class="kindatiny" data-round=({ round + 1 }) onclick=(format!("toggleRound({})", round + 1)) {
                            "Tap to shrink"
                        }
                    }
                }
                th class="topheader" {
                    i {
                        "Totals"
                    }
                }
            }
            tr {
                @let z_vec = ["Tee Time (CT)", "Holes Compl.", "Score"];
                @let z_len = z_vec.len();
                @for round in 0..max_len_of_tee_times_in_rounds {
                    // for each of the 4 columns, add tee time, holes completed, and score
                    @for a in 0..z_len {
                        @let c = round * z_vec.len() + a +1;
                        th class="sortable hideable" data-round=({
                            round + 1
                        })
                        onclick=(format!("sortTable('scores-table-{}', {})", group, c)) {
                            (z_vec[a])
                        }
                    }
                }
                @let d = max_len_of_tee_times_in_rounds * z_vec.len() +1;
                th class="sortable" onclick=(format!("sortTable('scores-table-{}', {})", group, d)) {
                    "Total"
                }
            }
        }
    }
}

fn render_score_detail(data: &ScoreData) -> Markup {
    html! {
        h4 class="playerdetails" { "Filter Details" }

        div class="playerdetails" {
            button class="playerdetailsbtn" onclick="toggleAllPlayersDetailDiv()" {
                "Click to show/hide details"
            }

            div class="playerdetailsdiv" style="display: none;" {
                p class="playerdetailsmsg" { "Showing details for all players. You can further filter by clicking links above." }

                @let grouped_scores = group_by_scores(data.score_struct.clone());
                @for (group, scores) in &grouped_scores {
                    @let max_len_of_tee_times = scores.iter().map(|score| score.detailed_statistics.tee_times.len()).max().unwrap_or(0);
                    table id=(format!("scores-table-{}", group)) {
                        (render_thead(max_len_of_tee_times, group) )
                        tbody {
                            @for score in scores {
                                tr class="playerrow" data-player=(score.bettor_name) {
                                    td { (score.bettor_name) }
                                    td { (score.golfer_name) }

                                    @let stats = &score.detailed_statistics;
                                    @for index in 0..max_len_of_tee_times {
                                        @let tee_time_len = stats.tee_times.len();
                                        @if index < tee_time_len {
                                            td class="cells hideable teetime" data-round=({ index + 1 }) {
                                                (stats.tee_times[index].val)
                                            }
                                            td class="cells hideable" data-round=({ index + 1 }) {
                                                @if index < stats.holes_completed.len() {
                                                    (stats.holes_completed[index].val)
                                                } @else {
                                                    "N/A"
                                                }
                                            }
                                            td class="cells" data-round=({ index + 1 }) {
                                                @if index < stats.scores.len() {
                                                    (stats.scores[index].val)
                                                } @else {
                                                    "N/A"
                                                }
                                            }
                                        } @else {
                                            td class="cells hideable" data-round=({ index + 1 }) { "N/A" }
                                            td class="cells hideable" data-round=({ index + 1 }) { "N/A" }
                                            td class="cells" data-round=({ index + 1 }) { "N/A" }
                                        }
                                    }
                                    td { (stats.total_score) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_drop_down_bar(data: &ScoreData, grouped_data: &AllBettorScoresByRound) -> Markup {
    html! {

        h3 class="playerbars" { "Filter" }

        @let summary_scores = crate::controller::score::group_by_bettor_golfer_round(&data.score_struct);
        @let summary_scores_x = grouped_data;

        div class="drop-down-bar-chart" {
            // Player selection dropdown
            div class="player-selection" {
                @for (idx, summary_score) in summary_scores_x.summary_scores.iter().enumerate() {
                    @let button_select = if idx == 0 { " selected" } else { "" };
                    button class=(format!("player-button{}", button_select)) data-player=(summary_score.bettor_name) {
                        (summary_score.bettor_name)
                    }
                }
            }

            // Graph rendering
            div class="chart-container" {
                // Draw the "T" structure
                div class="horizontal-line"  {}
                div class="vertical-line"  {}

                // this struct is groupd by bettor, one entry per bettor, so let's continue using that to iterate
                @for (idx, summary_score) in summary_scores_x.summary_scores.iter().enumerate() {
                    @let chart_visibility = if idx == 0 { " visible" } else { " hidden" };

                    div class=(format!("chart{}", chart_visibility)) data-player=(summary_score.bettor_name)  {
                        // now we want to pull all golfers for this bettor
                        @for (_golfer_idx, golfer) in summary_scores.detailed_scores.iter().filter(|golfer| golfer.bettor_name == summary_score.bettor_name).enumerate() {
                            // @let score = summary_scores
                            // go through each of the rounds, sorted by round number
                            @for (i, _round_entry)  in golfer.rounds.iter().enumerate()  {
                                @let score = golfer.scores.get(i).unwrap_or(&0);
                                div class="bar-row" style=(format!("--bar-width: {}rem;", score.abs() as f32 * 0.3))
                                    data-positive=(if *score >= 0 { "true" } else { "false" })
                                {
                                    div class="bar" style=(format!("width: {}rem;",score.abs() as f32 * 0.3,)) { }
                                    div class="bar-label" { (score) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
