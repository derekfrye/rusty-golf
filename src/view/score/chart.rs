use maud::{Markup, html};
use sql_middleware::middleware::ConfigAndPool;
use std::collections::BTreeMap;

use crate::model::{
    AllBettorScoresByRound, DetailedScore, SummaryDetailedScores, get_event_details,
};
use crate::view::score::types::{Bar, Direction, GolferBars};
use crate::view::score::utils::short_golfer_name;


/// # Errors
///
/// Will return `Err` if the database query fails
pub async fn preprocess_golfer_data(
    summary_scores_x: &AllBettorScoresByRound,
    detailed_scores: &[DetailedScore],
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<BTreeMap<String, Vec<GolferBars>>, Box<dyn std::error::Error>> {
    let mut bettor_golfers_map: BTreeMap<String, Vec<GolferBars>> = BTreeMap::new();

    let global_step_factor = get_event_details(config_and_pool, event_id)
        .await?
        .score_view_step_factor;

    let player_step_factors =
        crate::model::get_player_step_factors(config_and_pool, event_id).await?;

    for summary_score in summary_scores_x.summary_scores.iter() {
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
                    let width = (score.abs() as f32) * step_factor * scaling_factor;

                    if score < 0 {
                        bars.push(Bar {
                            score,
                            direction: Direction::Left,
                            start_position: 50.0 - cumulative_left - width,
                            width,
                            round: (round_idx + 1) as i32,
                        });
                        cumulative_left += width;
                    } else if score > 0 {
                        bars.push(Bar {
                            score,
                            direction: Direction::Right,
                            start_position: 50.0 + cumulative_right,
                            width,
                            round: (round_idx + 1) as i32,
                        });
                        cumulative_right += width;
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

    Ok(bettor_golfers_map)
}

/// # Errors
///
/// Will return `Err` if the database query fails
pub async fn render_drop_down_bar(
    summary_scores_x: &AllBettorScoresByRound,
    detailed_scores: &SummaryDetailedScores,
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<Markup, Box<dyn std::error::Error>> {
    let preprocessed_data = preprocess_golfer_data(
        summary_scores_x,
        &detailed_scores.detailed_scores,
        config_and_pool,
        event_id,
    )
    .await?;

    Ok(html! {
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
    })
}
