use maud::{Markup, html};
use std::collections::{BTreeMap, HashMap};
use std::hash::BuildHasher;

use crate::model::{
    AllBettorScoresByRound, BettorScoreByRound, DetailedScore, SummaryDetailedScores,
};
use crate::view::score::types::{Bar, Direction, GolferBars};
use crate::view::score::utils::short_golfer_name;

#[must_use]
pub fn preprocess_golfer_data_pure<S: BuildHasher>(
    summary_scores_x: &AllBettorScoresByRound,
    detailed_scores: &[DetailedScore],
    global_step_factor: f32,
    player_step_factors: &HashMap<(i64, String), f32, S>,
) -> BTreeMap<String, Vec<GolferBars>> {
    let mut bettor_golfers_map: BTreeMap<String, Vec<GolferBars>> = BTreeMap::new();

    for summary_score in &summary_scores_x.summary_scores {
        let mut golfers: Vec<GolferBars> = detailed_scores
            .iter()
            .filter(|golfer| golfer.bettor_name == summary_score.bettor_name)
            .enumerate()
            .map(|(golfer_idx, golfer)| {
                create_golfer_bars(golfer_idx, golfer, global_step_factor, player_step_factors)
            })
            .collect();

        // Restore alphabetical order by short name for stable, expected layout
        golfers.sort_by(|a, b| a.short_name.cmp(&b.short_name));
        bettor_golfers_map.insert(summary_score.bettor_name.clone(), golfers);
    }

    bettor_golfers_map
}

fn create_golfer_bars<S: BuildHasher>(
    golfer_idx: usize,
    golfer: &DetailedScore,
    global_step_factor: f32,
    player_step_factors: &HashMap<(i64, String), f32, S>,
) -> GolferBars {
    let short_name = short_golfer_name(&golfer.golfer_name);
    let total_score: isize = golfer.scores.iter().map(|&x| x as isize).sum();
    let step_factor = resolve_step_factor(golfer, global_step_factor, player_step_factors);
    let bars = build_bar_segments(golfer, step_factor);

    GolferBars {
        short_name,
        total_score,
        bars,
        is_even: golfer_idx.is_multiple_of(2),
    }
}

fn resolve_step_factor<S: BuildHasher>(
    golfer: &DetailedScore,
    global_step_factor: f32,
    player_step_factors: &HashMap<(i64, String), f32, S>,
) -> f32 {
    player_step_factors
        .get(&(golfer.golfer_espn_id, golfer.bettor_name.clone()))
        .copied()
        .unwrap_or(global_step_factor)
}

fn build_bar_segments(golfer: &DetailedScore, step_factor: f32) -> Vec<Bar> {
    let mut bars: Vec<Bar> = Vec::new();
    let mut cumulative_left = 0.0;
    let mut cumulative_right = 0.0;

    #[allow(clippy::cast_precision_loss)]
    let total_width: f32 = golfer
        .scores
        .iter()
        .map(|&score| (score.abs() as f32) * step_factor)
        .sum();

    let scaling_factor = if total_width > 100.0 {
        100.0 / total_width
    } else {
        1.0
    };

    for (round_idx, &score) in golfer.scores.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let mut width = (score.abs() as f32) * step_factor * scaling_factor;
        let round = i32::try_from(round_idx + 1).unwrap_or(1);
        match score.cmp(&0) {
            std::cmp::Ordering::Less => {
                bars.push(Bar {
                    score,
                    direction: Direction::Left,
                    start_position: 50.0 - cumulative_left - width,
                    width,
                    round,
                });
                cumulative_left += width;
            }
            std::cmp::Ordering::Greater => {
                bars.push(Bar {
                    score,
                    direction: Direction::Right,
                    start_position: 50.0 + cumulative_right,
                    width,
                    round,
                });
                cumulative_right += width;
            }
            std::cmp::Ordering::Equal => {
                // Render a tiny centered tick for even-par rounds so they remain visible
                let epsilon = 0.5_f32.min(100.0 * scaling_factor);
                width = epsilon;
                bars.push(Bar {
                    score,
                    direction: Direction::Right,
                    start_position: 50.0 - (width / 2.0),
                    width,
                    round,
                });
            }
        }
    }
    bars
}

#[must_use]
pub fn render_drop_down_bar_pure<S: BuildHasher>(
    summary_scores_x: &AllBettorScoresByRound,
    detailed_scores: &SummaryDetailedScores,
    global_step_factor: f32,
    player_step_factors: &HashMap<(i64, String), f32, S>,
) -> Markup {
    let preprocessed_data = preprocess_golfer_data_pure(
        summary_scores_x,
        &detailed_scores.detailed_scores,
        global_step_factor,
        player_step_factors,
    );

    let sorted_bettors = sorted_bettors(summary_scores_x);

    html! {
        h3 class="playerbars" { "Score by Player" }
        // Outer container with both old and new class names for CSS compatibility
        div class="drop-down-bar-chart player-bar-container" {
            // Old structure: player-selection for buttons
            div class="player-selection" { (render_player_buttons(&sorted_bettors)) }

            // Old structure: chart-container wraps visible charts
            div class="chart-container" { (render_chart_panels(&sorted_bettors, &preprocessed_data)) }
        }
    }
}

fn sorted_bettors(summary_scores_x: &AllBettorScoresByRound) -> Vec<&BettorScoreByRound> {
    let mut bettors = summary_scores_x.summary_scores.iter().collect::<Vec<_>>();
    bettors.sort_by(|a, b| a.bettor_name.cmp(&b.bettor_name));
    bettors
}

fn render_player_buttons(bettors: &[&BettorScoreByRound]) -> Markup {
    html! {
        @for (idx, summary_score) in bettors.iter().enumerate() {
            @let button_select = if idx == 0 { " selected" } else { "" };
            button class=(format!("player-button{}", button_select)) data-player=(summary_score.bettor_name) {
                (summary_score.bettor_name)
            }
        }
    }
}

fn render_chart_panels(
    bettors: &[&BettorScoreByRound],
    preprocessed_data: &BTreeMap<String, Vec<GolferBars>>,
) -> Markup {
    html! {
        @for (idx, summary_score) in bettors.iter().enumerate() {
            @let chart_visibility = if idx == 0 { " visible" } else { " hidden" };

            div class=(format!("chart{}", chart_visibility)) data-player=(summary_score.bettor_name)  {
                @let empty_vec = Vec::new();
                @let golfer_bars = preprocessed_data.get(&summary_score.bettor_name).unwrap_or(&empty_vec);
                @for golfer_bars in golfer_bars.iter() {
                    div class="golfer-bar-container chart-row" {
                        div class="golfer-label label-container" {
                            span class="golfer-name bar-label" {
                                (format!("{:<8}: {:<3}", &golfer_bars.short_name, golfer_bars.total_score))
                            }
                        }
                        div class=(format!("bar-row {}", if golfer_bars.is_even { "even" } else { "odd" })) {
                            // Provide both centerline styles used by old CSS
                            div class="progress-bar bars-container" {
                                div class="centerline horizontal-line" {}
                                div class="vertical-line" {}
                                @for bar in &golfer_bars.bars {
                                    @let bar_class = if bar.score == 0 {
                                        "bar zero"
                                    } else {
                                        match bar.direction {
                                            Direction::Left => "bar negative",
                                            Direction::Right => "bar positive",
                                        }
                                    };
                                    div class=(bar_class) data-round=(bar.round)
                                        style=(format!("left: {}%; width: {}%;",bar.start_position, bar.width)) {
                                        // Add bar-text for legacy selectors
                                        div class="bar-text" { "R" (bar.round) ": " (bar.score) }
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
