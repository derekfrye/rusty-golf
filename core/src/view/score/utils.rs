use crate::model::{ScoreDisplay, ScoresAndLastRefresh};
use crate::view::score::types::{BettorData, GolferData};
use maud::{Markup, html};
use std::collections::BTreeMap;

#[must_use]
pub fn short_golfer_name(golfer_name: &str) -> String {
    let parts: Vec<&str> = golfer_name.split_whitespace().collect();

    if let Some(first_initial) = parts.first().and_then(|s| s.chars().next()) {
        let last_name = parts.last().unwrap_or(&"");
        format!("{first_initial}. {last_name}")
    } else {
        golfer_name.to_string()
    }
}

#[must_use]
pub fn score_with_shape(score: &i32, disp: &ScoreDisplay) -> Markup {
    // Keep both legacy and semantic CSS classes for styling, but omit any glyphs.
    let (new_class, legacy_class) = match disp {
        ScoreDisplay::DoubleCondor => ("double-condor", "score-shape-doublecondor"),
        ScoreDisplay::Condor => ("condor", "score-shape-condor"),
        ScoreDisplay::Albatross => ("albatross", "score-shape-albatross"),
        ScoreDisplay::Eagle => ("eagle", "score-shape-eagle"),
        ScoreDisplay::Birdie => ("birdie", "score-shape-birdie"),
        ScoreDisplay::Par => ("par", "score-shape-par"),
        ScoreDisplay::Bogey => ("bogey", "score-shape-bogey"),
        ScoreDisplay::DoubleBogey => ("double-bogey", "score-shape-doublebogey"),
        ScoreDisplay::TripleBogey => ("triple-bogey", "score-shape-triplebogey"),
        ScoreDisplay::QuadrupleBogey => ("quadruple-bogey", "score-shape-quadruplebogey"),
        ScoreDisplay::QuintupleBogey => ("quintuple-bogey", "score-shape-quintuplebogey"),
        ScoreDisplay::SextupleBogey => ("sextuple-bogey", "score-shape-sextuplebogey"),
        ScoreDisplay::SeptupleBogey => ("septuple-bogey", "score-shape-septuplebogey"),
        ScoreDisplay::OctupleBogey => ("octuple-bogey", "score-shape-octuplebogey"),
        ScoreDisplay::NonupleBogey => ("nonuple-bogey", "score-shape-nonuplebogey"),
        ScoreDisplay::DodecupleBogey => ("dodecuple-bogey", "score-shape-dodecuplebogey"),
    };

    let combined_classes = format!("{legacy_class} {new_class}");

    html! {
        span class=(combined_classes) { (score) }
    }
}

#[must_use]
pub fn scores_and_last_refresh_to_line_score_tables(
    scores_and_last_refresh: &ScoresAndLastRefresh,
) -> Vec<BettorData> {
    // Use BTreeMap for deterministic alphabetical ordering and merge per-golfer data
    type GolferScoreData = (Vec<crate::model::LineScore>, Vec<crate::model::StringStat>);
    type GolferMap = BTreeMap<String, GolferScoreData>;
    let mut grouped: BTreeMap<String, GolferMap> = BTreeMap::new();

    for s in &scores_and_last_refresh.score_struct {
        let bettor_name = &s.bettor_name;
        let golfer_name = &s.golfer_name;
        let linescores = &s.detailed_statistics.line_scores;
        let teetimes = &s.detailed_statistics.tee_times;

        grouped
            .entry(bettor_name.clone())
            .or_default()
            .entry(golfer_name.clone())
            .or_default()
            .0
            .extend(linescores.iter().cloned());

        grouped
            .entry(bettor_name.clone())
            .or_default()
            .entry(golfer_name.clone())
            .or_default()
            .1
            .extend(teetimes.iter().cloned());
    }

    let mut bettor_data_vec = Vec::new();
    for (bettor_name, golfer_map) in grouped {
        let mut golfer_data_vec = Vec::new();
        for (golfer_name, (mut linescores, tee_times)) in golfer_map {
            // Ensure a stable in-table order by (round, hole)
            linescores.sort_by_key(|ls| (ls.round, ls.hole));
            golfer_data_vec.push(GolferData {
                golfer_name,
                linescores,
                tee_times,
            });
        }

        bettor_data_vec.push(BettorData {
            bettor_name,
            golfers: golfer_data_vec,
        });
    }

    bettor_data_vec
}
