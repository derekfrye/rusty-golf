use std::collections::BTreeMap;

use crate::get_title_and_score_view_conf_from_db;
use crate::model::{
    get_scores_from_db, AllBettorScoresByRound, DetailedScore, LineScore, RefreshSource, ScoreData, ScoreDisplay, ScoresAndLastRefresh, StringStat, SummaryDetailedScores
};

use maud::{ html, Markup };
use sql_middleware::middleware::ConfigAndPool;

pub async fn render_scores_template(
    data: &ScoreData,
    expanded: bool,
    config_and_pool: &ConfigAndPool,
    event_id: i32
) -> Result<Markup, Box<dyn std::error::Error>> {
    let summary_scores_x = crate::controller::score::group_by_bettor_name_and_round(
        &data.score_struct
    );
    let detailed_scores = crate::controller::score::group_by_bettor_golfer_round(
        &data.score_struct
    );

    let golfer_scores_for_line_score_render = get_scores_from_db(
        config_and_pool,
        event_id,
        RefreshSource::Db
    ).await?;
    // map to BettorData
    let bettor_struct = scores_and_last_refresh_to_line_score_tables(
        &golfer_scores_for_line_score_render
    );

    let refresh_data = RefreshData {
        last_refresh: data.last_refresh.clone(),
        last_refresh_source: data.last_refresh_source.clone(),
    };

    Ok(
        html! {
        (render_scoreboard(data))
        @if expanded {
            (render_summary_scores(&summary_scores_x))
        }
        // (render_stacked_bar_chart(data))
        (render_drop_down_bar(&summary_scores_x, &detailed_scores, config_and_pool, event_id).await?)
        (render_line_score_tables(&bettor_struct, refresh_data))
        // (render_tee_time_detail(data))
    }
    )
}

fn render_scoreboard(data: &ScoreData) -> Markup {
    html! {
        @if !data.score_struct.is_empty(){

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



// Structure to hold individual bar information
struct Bar {
    score: i32,
    direction: Direction,
    start_position: f32, // In percentage
    width: f32, // Width of the bar in percentage
    round: i32,
}

// Enum to represent the direction of the bar
enum Direction {
    Left,
    Right,
}

// Structure to hold golfer information along with precomputed bars
struct GolferBars {
    // name: String,
    short_name: String,
    total_score: isize,
    bars: Vec<Bar>,
    is_even: bool, // For alternating row colors
}

async fn preprocess_golfer_data(
    summary_scores_x: &AllBettorScoresByRound,
    detailed_scores: &Vec<DetailedScore>,
    config_and_pool: &ConfigAndPool,
    event_id: i32
) -> Result<BTreeMap<String, Vec<GolferBars>>, Box<dyn std::error::Error>> {
    let mut bettor_golfers_map: BTreeMap<String, Vec<GolferBars>> = BTreeMap::new();

    let step_factor = get_title_and_score_view_conf_from_db(
        config_and_pool,
        event_id
    ).await?.score_view_step_factor;

    for (_bettor_idx, summary_score) in summary_scores_x.summary_scores.iter().enumerate() {
        let mut golfers: Vec<GolferBars> = detailed_scores
            .iter()
            .filter(|golfer| golfer.bettor_name == summary_score.bettor_name)
            .enumerate()
            .map(|(golfer_idx, golfer)| {
                let short_name = short_golfer_name(&golfer.golfer_name);

                let total_score: isize = golfer.scores
                    .iter()
                    .map(|&x| x as isize)
                    .sum();

                // Calculate bars
                let mut bars: Vec<Bar> = Vec::new();
                let mut cumulative_left = 0.0;
                let mut cumulative_right = 0.0;

                // First, calculate total required width
                let total_width: f32 = golfer.scores
                    .iter()
                    .map(|&score| (score.abs() as f32) * step_factor)
                    .sum();

                // Determine scaling factor if total_width exceeds 100%
                let scaling_factor = if total_width > 100.0 { 100.0 / total_width } else { 1.0 };

                for &score in &golfer.scores {
                    let direction = if score > 0 { Direction::Right } else { Direction::Left };
                    // Convert score to percentage with scaling
                    let width =
                        ((if score.abs() == 0 { 1 } else { score.abs() }) as f32) *
                        step_factor *
                        scaling_factor;

                    let start_position = match direction {
                        Direction::Right => {
                            let pos = 50.0 + cumulative_right;
                            cumulative_right += width + 0.1; // Add a small gap
                            pos
                        }
                        Direction::Left => {
                            cumulative_left += width + 0.1; // Add a small gap
                            50.0 - cumulative_left
                        }
                    };

                    bars.push(Bar {
                        score,
                        direction,
                        start_position,
                        width,
                        round: (bars.len() as i32) + 1,
                    });
                }

                GolferBars {
                    short_name,
                    total_score,
                    bars,
                    is_even: golfer_idx % 2 == 0,
                }
            })
            .collect();

        // Sort the golfers by `short_name`
        golfers.sort_by(|a, b| a.short_name.cmp(&b.short_name));

        // Insert the sorted golfers into the map
        bettor_golfers_map.insert(summary_score.bettor_name.clone(), golfers);
    }

    Ok(bettor_golfers_map)
}

async fn render_drop_down_bar(
    grouped_data: &AllBettorScoresByRound,
    detailed_scores: &SummaryDetailedScores,
    config_and_pool: &ConfigAndPool,
    event_id: i32
) -> Result<Markup, Box<dyn std::error::Error>> {
    // Preprocess the data
    let preprocessed_data = preprocess_golfer_data(
        &grouped_data,
        &detailed_scores.detailed_scores,
        config_and_pool,
        event_id
    ).await?;

    Ok(
        html! {
        h3 class="playerbars" { "Score by Player" }

        @let sorted_x = {
            let mut vec: Vec<_> = grouped_data.summary_scores.iter().collect();
                vec.sort_by_key(|score| &score.bettor_name);
                vec
        };

        div class="drop-down-bar-chart" {
            // Player selection dropdown
            div class="player-selection" {                

                @for (idx, summary_score) in sorted_x.iter().enumerate() {
                    @let button_select = if idx == 0 { " selected" } else { "" };
                    button class=(format!("player-button{}", button_select)) data-player=(summary_score.bettor_name) {
                        (summary_score.bettor_name)
                    }
                }
            }

            // Chart rendering
            div class="chart-container" {

                // Iterate over each bettor
                @for (bettor_idx, summary_score) in sorted_x.iter().enumerate() {

                    @let chart_visibility = if bettor_idx == 0 { " visible" } else { " hidden" };
                    div class=(format!("chart {}", chart_visibility)) data-player=(summary_score.bettor_name)  {
                        // Iterate over each preprocessed golfer for the current bettor
                        @for (_golfer_idx, golfer_bars) in preprocessed_data.get(&summary_score.bettor_name).unwrap_or(&Vec::new()).iter().enumerate() {

                            div class="chart-row" {

                                div class="label-container" {
                                    div class="bar-label" {
                                        (format!("{:<8}: {:<3}", &golfer_bars.short_name, golfer_bars.total_score))
                                    }
                                }
                                // Create bar-row with alternating background
                                div class=(format!("bar-row {}", if golfer_bars.is_even { "even" } else { "odd" })) {
                                    // Draw the "T" structure
                                    div class="horizontal-line"  {}
                                    div class="vertical-line"  {}
                                    // Bars Container
                                    div class="bars-container" {
                                        @for bar in &golfer_bars.bars {
                                            div class=(match bar.direction {
                                                    Direction::Right => "bar positive",
                                                    Direction::Left => match bar.score {
                                                        0 => "bar zero",
                                                        _ => "bar negative",
                                                    }
                                                })
                                            style=(format!("left: {}%; width: {}%;",bar.start_position, bar.width)) {

                                                div class="bar-text" {
                                                    "R"(bar.round)": "( bar.score)
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
        }
    }
    )
}

#[derive(Debug)]
pub struct BettorData {
    pub bettor_name: String,
    pub golfers: Vec<GolferData>,
}

pub struct RefreshData {
    pub last_refresh: String,
    pub last_refresh_source: RefreshSource,
}

#[derive(Debug)]
pub struct GolferData {
    pub golfer_name: String,
    pub linescores: Vec<LineScore>,
    pub tee_times: Vec<StringStat>,
}

pub fn render_line_score_tables(bettors: &Vec<BettorData>, refresh_data: RefreshData) -> Markup {
    html! {

        h3 class="playerbars" { "Score by Golfer" }

        @for (idx, bettor) in bettors.iter().enumerate() {

            // We'll hide all but the first by default, or all hidden by default
            // depending on your preference. 
            @let visibility_class = if idx == 0 { "linescore-container visible" } 
                                    else { "linescore-container hidden" };

            // Container for all the golfer tables for this bettor
            div class=(visibility_class) data-player=(bettor.bettor_name) {
                @for golfer in &bettor.golfers {
                    @let unique_rounds = {
                        // Collect unique round numbers for the round buttons
                        // increment by 1 since we're 0 based in the database
                        let mut rds: Vec<i32> = golfer.linescores
                            .iter()
                            .map(|ls| ls.round + 1)
                            .collect();
                        rds.sort();
                        rds.dedup();
                        rds
                    };

                    // Build a table
                    table class="linescore-table" {
                        thead {
                            // First header row:
                            //  - First column: Golfer name, rowspan=2
                            //  - Second column: colSpan=3, which holds the round buttons
                            tr {
                                th class="topheader" {
                                    (short_golfer_name(&golfer.golfer_name))
                                }
                                th colspan="2" class="topheader" {
                                    
                                    @for rd in unique_rounds.iter().take(2) {

                                        @let is_round_one = *rd == 1;
                                        @let row_class = if is_round_one { "linescore-round-button selected" } else { "linescore-round-button" };

                                        button class=(row_class) data-round=(rd) {
                                            "R" (rd)
                                        }
                                        " "  // small space
                                    }
                                }
                            }
                            // Second header row:
                            tr {
                                th class="topheader"  {
                                    @for rd in unique_rounds.iter() {

                                        @let is_round_one = *rd == 1;
                                        @let row_class = if is_round_one { "topheader" } else { "topheader hidden" };

                                    
                                        @if golfer.tee_times.len() > (*rd - 1) as usize {
                                            div class=(row_class) data-round=(rd) { (&golfer.tee_times[(*rd - 1) as usize].val) }
                                        }
                                    }
                                }
                                th colspan="2" class="topheader" {
                                    
                                    @for rd in unique_rounds.iter().skip(2) {

                                        @let is_round_one = *rd == 1;
                                        @let row_class = if is_round_one { "linescore-round-button selected" } else { "linescore-round-button" };

                                        button class=(row_class) data-round=(rd) {
                                            "R" (rd)
                                        }
                                        " "  // small space
                                    }
                                }
                            }
                            // third header row:
                            tr {
                                // th { "Rd" }
                                th { "Hole" }
                                th { "Par" }
                                th { "Strokes" }
                            }
                        }
                        tbody {
                            // Sort linescores by (round, hole) so they appear in a natural order
                            @let all_scores = {
                                let mut scores = golfer.linescores.clone();
                                scores.sort_by_key(|ls| (ls.round, ls.hole));
                                scores
                            };

                            @for (_, ls) in all_scores.iter().enumerate() {

                                @let is_round_one = ls.round + 1 == 1;
                                @let row_class = if is_round_one { "" } else { "hidden" };

                                tr class=(row_class) data-round=(ls.round + 1) {
                                    // td {
                                    //     (ls.round + 1)
                                    // }
                                    td {
                                        (ls.hole)
                                    }
                                    td {
                                        (ls.par)
                                    }
                                    // The "Strokes" cell with a shape if needed
                                    td class="score-cell" {
                                        (score_with_shape(&ls.score, &ls.score_display))
                                    }
                                }
                            }
                            @if !all_scores.is_empty() {
                                @let scores_by_round = {
                                    let mut x = BTreeMap::new();
                                    for ls in all_scores.iter() {
                                        x.entry(ls.round)
                                        .and_modify(|total| *total += ls.score - ls.par)
                                        .or_insert(ls.score - ls.par);
                                    }
                                    x
                                };
                                @for (round, total_score) in scores_by_round.iter() {
                                    @let is_round_one = *round == 0;  // Since ls.round is 0-based
                                    @let row_class = if is_round_one { "linescore-total" } else { "linescore-total hidden" };
                            
                                    tr class=(row_class) data-round=(round + 1) {
                                        td data-round=(round + 1) colspan="2" class="linescore-total-cell" { 
                                            "Total:" 
                                        }
                                        td data-round=(round + 1) class="linescore-total-cell" {
                                            (total_score)
                                        }
                                    }
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

fn short_golfer_name(golfer_name: &str) -> String {
    let short_name_x = golfer_name.split_whitespace().into_iter();
    let shortname = if short_name_x.clone().count() >= 2 {
        short_name_x.clone().nth(1).unwrap_or(" ").to_string()
    } else {
        golfer_name.to_string()
    };

    format!(
        "{}. {}",
        golfer_name.chars().take(1).collect::<String>(),
        shortname.chars().take(5).collect::<String>()
    )
}

/// Helper that returns a sub‐Markup for the strokes cell, optionally wrapping
/// the numeric score in a circle or square depending on `ScoreDisplay`.
fn score_with_shape(score: &i32, disp: &ScoreDisplay) -> Markup {
    // For convenience, define CSS classes for each shape.
    // (See the CSS snippet below.)
    let class_name = match disp {
        ScoreDisplay::Birdie => "score-shape-birdie",
        ScoreDisplay::Eagle => "score-shape-eagle",
        ScoreDisplay::Bogey => "score-shape-bogey",
        ScoreDisplay::DoubleBogey => "score-shape-doublebogey",
        // For anything else, we won't wrap it in a shape.
        ScoreDisplay::Par => "score-shape-par",
        // ... match other variants if you want special styles ...

        _ => "score-shape-par",
    };

    if class_name.is_empty() {
        // Just return the raw numeric score
        html! {
            (score)
        }
    } else {
        // Wrap the numeric score in a styled <span>
        html! {
            span class=(class_name) {
                (score)
            }
        }
    }
}

fn scores_and_last_refresh_to_line_score_tables(
    scores_and_refresh: &ScoresAndLastRefresh
) -> Vec<BettorData> {
    // We'll group by bettor_name -> golfer_name -> Vec<LineScore>.
    // Use BTreeMap for a predictable sort order (alphabetical).
    let mut grouped: BTreeMap<String, BTreeMap<String, (Vec<LineScore>, Vec<StringStat>)>> = BTreeMap::new();

    // Iterate over every `Scores` entry in the structure
    for s in &scores_and_refresh.score_struct {
        // Extract fields
        let bettor_name = &s.bettor_name;
        let golfer_name = &s.golfer_name;
        let linescores = &s.detailed_statistics.line_scores;
        let teetimes = &s.detailed_statistics.tee_times;

        // Insert into the map
        grouped
            .entry(bettor_name.clone())
            .or_default()
            .entry(golfer_name.clone())
            .or_default()
            .0.extend(linescores.iter().cloned());
        grouped
            .entry(bettor_name.clone())
            .or_default()
            .entry(golfer_name.clone())
            .or_default()
            .1.extend(teetimes.iter().cloned());
    }

    // Now convert that grouped map into the final Vec<BettorData>.
    let mut bettor_data_vec = Vec::new();

    for (bettor_name, golfer_map) in grouped {
        let mut golfer_data_vec = Vec::new();

        for (golfer_name, lscores) in golfer_map {
            golfer_data_vec.push(GolferData {
                golfer_name,
                linescores: lscores.0,
                tee_times: lscores.1,
            });
        }

        bettor_data_vec.push(BettorData {
            bettor_name,
            golfers: golfer_data_vec,
        });
    }

    bettor_data_vec
}
