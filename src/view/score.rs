use crate::controller::score::{group_by_bettor_name_and_round, group_by_scores};
use crate::model::ScoreData;

use maud::{html, Markup};

pub fn render_scores_template(data: &ScoreData) -> Markup {
    html! {
        @if !data.score_struct.is_empty() {
            (render_scoreboard(data))
            (render_summary_scores(data))
            (render_stacked_bar_chart(data))
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

fn render_summary_scores(data: &ScoreData) -> Markup {
    html! {
        @let summary_scores = group_by_bettor_name_and_round(&data.score_struct);

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
                                @let score = summary_score.new_scores[idx];
                                td { (score) }
                            }
                            @let total = summary_score.new_scores.iter().sum::<isize>();
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
        h3 { "Details" }
         @let grouped_scores = group_by_scores(data.score_struct.clone());
        @for (group, scores) in &grouped_scores {
            @let max_len_of_tee_times = scores.iter().map(|score| score.detailed_statistics.tee_times.len()).max().unwrap_or(0);
            table id=(format!("scores-table-{}", group)) {
                (render_thead(max_len_of_tee_times, group) )
                tbody {
                    @for score in scores {
                        tr {
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

fn render_stacked_bar_chart(data: &ScoreData) -> Markup {
    html! {
        @let summary_scores = group_by_bettor_name_and_round(&data.score_struct);

        @if !summary_scores.summary_scores.is_empty() {
            h3 { "Summary" }

            div class="stacked-bar-chart" {

                @let min_score: isize = summary_scores.summary_scores.iter()
                    .map(|summary_score| summary_score.new_scores.iter().sum::<isize>())
                    .collect::<Vec<isize>>()
                    .into_iter().min().unwrap_or(0).abs();

                @let max_scorea: isize = summary_scores.summary_scores.iter()
                    .map(|summary_score| summary_score.new_scores.iter().sum::<isize>())
                    .collect::<Vec<isize>>()
                    .into_iter().max().unwrap_or(0).abs();

                @let max_ticks = (max_scorea.max(min_score) + 19) / 20 * 20; // Round up to the nearest 20
                @let vert_axis_height = (max_ticks as f64) / 10.0;
                @let max_ticks_scaled = ((max_ticks as f64) * 0.3 * 10.0).round() / 10.0;

                div class="vertical-axis" style=(format!("position: absolute; width: 100%; height: {}rem; display: flex; flex-direction: column-reverse; gap: 0; z-index: 1;"
                    , max_ticks_scaled )) {

                    // need this here to make bottom of first tick line up with bottom of first bar
                    div class="bar-group-label" {  }

                    @for tick in (0..=max_ticks).step_by(10) {
                        div class="axis-line" style=(format!("position: relative; height: {}rem; display: flex; align-items: center;", vert_axis_height)) {
                            span class="tick-label" style=("position: absolute; left: -0.05rem; font-size: 0.8rem;") {
                                (tick)
                            }
                            div class="tick-line" style=("width: 100%; height: 0.05rem; background-color: #ddd; ") {}
                        }
                    }

                }

                div class="chart-body" style=(format!("position: relative; width: 100%; height: {}rem; display: flex; align-items: flex-end; z-index: 2;", max_ticks_scaled)) {
                    @for summary_score in &summary_scores.summary_scores {
                        div class="bar-group" {
                            @let total_score: isize = summary_score.new_scores.iter().sum();

                            div class="total-label" { (total_score) }

                            div class="bars" style=(format!("display: flex; flex-direction: column-reverse; gap: 0.06rem;")) {
                                @for (idx, _round) in summary_score.computed_rounds.iter().enumerate() {
                                    @let score = summary_score.new_scores[idx];
                                    @let score_scaled = ((score.abs() as f64) / (max_ticks as f64) * max_ticks_scaled).round();
                                    div class="bar" style=(format!(
                                        "height: {}rem; background-color: {};",
                                            score_scaled
                                        ,
                                        match idx {
                                            0 => "#007BFF",
                                            1 => "#FF5733",
                                            2 => "#33FF57",
                                            3 => "#FFC300",
                                            4 => "#C70039",
                                            5 => "#900C3F",
                                            6 => "#581845",
                                            _ => "#DAF7A6",
                                        }
                                    )) { }
                                }
                            }

                            div class="bar-group-label" { (summary_score.bettor_name) }
                        }
                    }
                }
            }
        }
    }
}
