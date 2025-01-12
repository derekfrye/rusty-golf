use std::collections::HashMap;

use crate::controller::score::group_by_scores;
use crate::model::{AllBettorScoresByRound, DetailedScore, ScoreData, SummaryDetailedScores};

use maud::{html, Markup};

pub fn render_scores_template(data: &ScoreData, expanded: bool) -> Markup {
    let summary_scores_x =
        crate::controller::score::group_by_bettor_name_and_round(&data.score_struct);
        let detailed_scores = crate::controller::score::group_by_bettor_golfer_round(&data.score_struct);

    html! {
        @if !data.score_struct.is_empty() {
            (render_scoreboard(data))
            @if expanded {
                (render_summary_scores(&summary_scores_x))
            }
            // (render_stacked_bar_chart(data))
            (render_drop_down_bar(data, &summary_scores_x, &detailed_scores))
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

// Structure to hold individual bar information
struct Bar {
    score: i32,
    direction: Direction,
    start_position: f32, // In percentage
    width: f32,           // Width of the bar in percentage
}

// Enum to represent the direction of the bar
enum Direction {
    Left,
    Right,
}

// Structure to hold golfer information along with precomputed bars
struct GolferBars {
    name: String,
    short_name: String,
    total_score: isize,
    bars: Vec<Bar>,
    is_even: bool, // For alternating row colors
}

fn preprocess_golfer_data(
    summary_scores_x: &AllBettorScoresByRound,
    detailed_scores: &Vec<DetailedScore>, // Assuming this is your detailed scores structure
) -> HashMap<String, Vec<GolferBars>> {
    let mut bettor_golfers_map: HashMap<String, Vec<GolferBars>> = HashMap::new();

    for (bettor_idx, summary_score) in summary_scores_x.summary_scores.iter().enumerate() {
        let golfers: Vec<GolferBars> = detailed_scores
            .iter()
            .filter(|golfer| golfer.bettor_name == summary_score.bettor_name)
            .enumerate()
            .map(|(golfer_idx, golfer)| {
                let short_name = golfer.golfer_name.chars().take(5).collect::<String>();

                let total_score: isize = golfer.scores.iter().map(|&x| x as isize).sum();

                // Calculate bars
                let mut bars: Vec<Bar> = Vec::new();
                let mut cumulative_left = 0.0;
                let mut cumulative_right = 0.0;

                for &score in &golfer.scores {
                    let direction = if score >= 0 {
                        Direction::Right
                    } else {
                        Direction::Left
                    };
                    // Convert score to percentage. Adjust the multiplier as needed.
                    // For example, 1 score unit = 2%
                    let width = (score.abs() as f32) * 2.0;

                    let start_position = match direction {
                        Direction::Right => {
                            let pos = 52.0 + cumulative_right;
                            cumulative_right += width;
                            pos
                        }
                        Direction::Left => {
                            cumulative_left += width;
                            48.0 - cumulative_left
                        }
                    };

                    bars.push(Bar {
                        score,
                        direction,
                        start_position,
                        width,
                    });
                }

                GolferBars {
                    name: golfer.golfer_name.clone(),
                    short_name,
                    total_score,
                    bars,
                    is_even: golfer_idx % 2 == 0,
                }
            })
            .collect();

        bettor_golfers_map.insert(summary_score.bettor_name.clone(), golfers);
    }

    bettor_golfers_map
}

fn render_drop_down_bar(
    data: &ScoreData,
    grouped_data: &AllBettorScoresByRound,
    detailed_scores: &SummaryDetailedScores,
) -> Markup {
    // Preprocess the data
    let preprocessed_data = preprocess_golfer_data(&grouped_data, &detailed_scores.detailed_scores);

    html! {
        h3 class="playerbars" { "Filter" }

        div class="drop-down-bar-chart" {
            // Player selection dropdown
            div class="player-selection" {
                @for (idx, summary_score) in grouped_data.summary_scores.iter().enumerate() {
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

                // Iterate over each bettor
                @for (bettor_idx, summary_score) in grouped_data.summary_scores.iter().enumerate() {
                    @let chart_visibility = if bettor_idx == 0 { " visible" } else { " hidden" };

                    div class=(format!("chart{}", chart_visibility)) data-player=(summary_score.bettor_name)  {
                        // Iterate over each preprocessed golfer for the current bettor
                        @for (golfer_idx, golfer_bars) in preprocessed_data.get(&summary_score.bettor_name).unwrap_or(&Vec::new()).iter().enumerate() {
                            // Create bar-row with alternating background
                            div class=(if golfer_bars.is_even { "bar-row even" } else { "bar-row odd" }) {
                                // Bar-label: first 5 characters of golfer_name and total score
                                div class="bar-label" {
                                    (format!("{}: {}", &golfer_bars.short_name, golfer_bars.total_score))
                                }

                                // Bars container
                                div class="bars-container" {
                                    @for bar in &golfer_bars.bars {
                                        div class=(match bar.direction {
                                            Direction::Right => "bar positive",
                                            Direction::Left => "bar negative",
                                        })
                                        style=(format!(
                                            "left: {}%; width: {}%;",
                                            bar.start_position, bar.width
                                        )) {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
