use crate::model::{ScoreDisplay, ScoresAndLastRefresh};
use crate::view::score::types::{BettorData, GolferData};
use maud::{Markup, html};
use std::collections::HashMap;

pub fn short_golfer_name(golfer_name: &str) -> String {
    let parts: Vec<&str> = golfer_name.split_whitespace().collect();

    if parts.len() >= 2 {
        let first_initial = parts[0].chars().next().unwrap_or(' ');
        let last_name = parts[parts.len() - 1];
        format!("{first_initial}. {last_name}")
    } else {
        golfer_name.to_string()
    }
}

pub fn score_with_shape(score: &i32, disp: &ScoreDisplay) -> Markup {
    let (shape, color) = match disp {
        ScoreDisplay::DoubleCondor => ("◆", "double-condor"),
        ScoreDisplay::Condor => ("◆", "condor"),
        ScoreDisplay::Albatross => ("◆", "albatross"),
        ScoreDisplay::Eagle => ("◆", "eagle"),
        ScoreDisplay::Birdie => ("●", "birdie"),
        ScoreDisplay::Par => ("●", "par"),
        ScoreDisplay::Bogey => ("▲", "bogey"),
        ScoreDisplay::DoubleBogey => ("▲", "double-bogey"),
        ScoreDisplay::TripleBogey => ("▲", "triple-bogey"),
        ScoreDisplay::QuadrupleBogey => ("▲", "quadruple-bogey"),
        ScoreDisplay::QuintupleBogey => ("▲", "quintuple-bogey"),
        ScoreDisplay::SextupleBogey => ("▲", "sextuple-bogey"),
        ScoreDisplay::SeptupleBogey => ("▲", "septuple-bogey"),
        ScoreDisplay::OctupleBogey => ("▲", "octuple-bogey"),
        ScoreDisplay::NonupleBogey => ("▲", "nonuple-bogey"),
        ScoreDisplay::DodecupleBogey => ("▲", "dodecuple-bogey"),
    };

    html! {
        span class=(color) { (shape) " " (score) }
    }
}

pub fn scores_and_last_refresh_to_line_score_tables(
    scores_and_last_refresh: &ScoresAndLastRefresh,
) -> Vec<BettorData> {
    let mut bettor_map: HashMap<String, Vec<GolferData>> = HashMap::new();

    for score in &scores_and_last_refresh.score_struct {
        let golfer_data = GolferData {
            golfer_name: score.golfer_name.clone(),
            linescores: score.detailed_statistics.line_scores.clone(),
            tee_times: score.detailed_statistics.tee_times.clone(),
        };

        bettor_map
            .entry(score.bettor_name.clone())
            .or_default()
            .push(golfer_data);
    }

    bettor_map
        .into_iter()
        .map(|(bettor_name, golfers)| BettorData {
            bettor_name,
            golfers,
        })
        .collect()
}
