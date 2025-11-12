use maud::{Markup, html};
use std::collections::{BTreeMap, HashMap};

use crate::model::{AllBettorScoresByRound, DetailedScore, SummaryDetailedScores};
use crate::view::score::types::{Bar, Direction, GolferBars};
use crate::view::score::utils::short_golfer_name;

#[must_use]
pub fn preprocess_golfer_data_pure(
    summary_scores_x: &AllBettorScoresByRound,
    detailed_scores: &[DetailedScore],
    global_step_factor: f32,
    player_step_factors: &HashMap<(i64, String), f32>,
) -> BTreeMap<String, Vec<GolferBars>> {
    let mut bettor_golfers_map: BTreeMap<String, Vec<GolferBars>> = BTreeMap::new();

    for summary_score in &summary_scores_x.summary_scores {
        let mut golfers: Vec<GolferBars> = detailed_scores
            .iter()
            .filter(|golfer| golfer.bettor_name == summary_score.bettor_name)
            .enumerate()
            .map(|(golfer_idx, golfer)| {
                let short_name = short_golfer_name(&golfer.golfer_name);
                let total_score: isize = golfer.scores.iter().map(|&x| x as isize).sum();

                let mut found_step_factor = None;

                if let Some(value) =
                    player_step_factors.get(&(golfer.golfer_espn_id, golfer.bettor_name.clone()))
                {
                    found_step_factor = Some(*value);
                }

                let step_factor = found_step_factor.unwrap_or(global_step_factor);

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
                    let width = (score.abs() as f32) * step_factor * scaling_factor;

                    match score.cmp(&0) {
                        std::cmp::Ordering::Less => {
                            bars.push(Bar {
                                score,
                                direction: Direction::Left,
                                start_position: 50.0 - cumulative_left - width,
                                width,
                                round: i32::try_from(round_idx + 1).unwrap_or(1),
                            });
                            cumulative_left += width;
                        }
                        std::cmp::Ordering::Greater => {
                            bars.push(Bar {
                                score,
                                direction: Direction::Right,
                                start_position: 50.0 + cumulative_right,
                                width,
                                round: i32::try_from(round_idx + 1).unwrap_or(1),
                            });
                            cumulative_right += width;
                        }
                        std::cmp::Ordering::Equal => {
                            // score == 0, do nothing
                        }
                    }
                }

                GolferBars {
                    short_name,
                    total_score,
                    bars,
                    is_even: golfer_idx % 2 == 0,
                }
            })
            .collect();

        golfers.sort_by(|a, b| a.total_score.cmp(&b.total_score));
        bettor_golfers_map.insert(summary_score.bettor_name.clone(), golfers);
    }

    bettor_golfers_map
}

#[must_use]
pub fn render_drop_down_bar_pure(
    summary_scores_x: &AllBettorScoresByRound,
    detailed_scores: &SummaryDetailedScores,
    global_step_factor: f32,
    player_step_factors: &HashMap<(i64, String), f32>,
) -> Markup {
    let preprocessed_data = preprocess_golfer_data_pure(
        summary_scores_x,
        &detailed_scores.detailed_scores,
        global_step_factor,
        player_step_factors,
    );

    html! {
        h3 class="playerbars" { "Score by Player" }
        div class="player-bar-container" {
            @for (idx, summary_score) in summary_scores_x.summary_scores.iter().enumerate() {
                @let button_select = if idx == 0 { " selected" } else { "" };

                button class=(format!("player-button{}", button_select)) data-player=(summary_score.bettor_name) {
                    (summary_score.bettor_name)
                }
            }

            @for (idx, summary_score) in summary_scores_x.summary_scores.iter().enumerate() {
                @let chart_visibility = if idx == 0 { "visible" } else { "hidden" };

                div class=(format!("chart {}", chart_visibility)) data-player=(summary_score.bettor_name)  {
                    @let empty_vec = Vec::new();
                    @let golfer_bars = preprocessed_data.get(&summary_score.bettor_name).unwrap_or(&empty_vec);
                    @for golfer_bars in golfer_bars.iter() {
                        div class="golfer-bar-container" {
                            div class="golfer-label" {
                                span class="golfer-name" {
                                    (format!("{:<8}: {:<3}", &golfer_bars.short_name, golfer_bars.total_score))
                                }
                            }
                            div class=(format!("bar-row {}", if golfer_bars.is_even { "even" } else { "odd" })) {
                                div class="progress-bar" {
                                    div class="centerline" {}
                                    @for bar in &golfer_bars.bars {
                                        @let bar_class = match bar.direction {
                                            Direction::Left => "bar negative",
                                            Direction::Right => "bar positive",
                                        };
                                        div class=(bar_class) data-round=(bar.round)
                                            style=(format!("left: {}%; width: {}%;",bar.start_position, bar.width)) {
                                            (bar.score)
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
