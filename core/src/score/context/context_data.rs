use std::collections::HashMap;

use crate::model::format_time_ago_for_score_view;
use crate::model::{Bettors, RefreshSource, ScoreData, ScoresAndLastRefresh};

#[must_use]
pub fn score_data_from_scores(scores: &ScoresAndLastRefresh) -> ScoreData {
    let cache_hit = matches!(
        scores.last_refresh_source,
        RefreshSource::Db | RefreshSource::R2 | RefreshSource::Kv | RefreshSource::Memory
    );
    score_data_from_scores_with_cache(scores, cache_hit)
}

#[must_use]
pub fn score_data_from_scores_with_cache(
    scores: &ScoresAndLastRefresh,
    cache_hit: bool,
) -> ScoreData {
    let mut totals: HashMap<String, i32> = HashMap::new();
    for golfer in &scores.score_struct {
        *totals.entry(golfer.bettor_name.clone()).or_insert(0) +=
            golfer.detailed_statistics.total_score;
    }

    let mut bettors: Vec<Bettors> = totals
        .into_iter()
        .map(|(name, total)| Bettors {
            bettor_name: name,
            total_score: total,
            scoreboard_position_name: String::new(),
            scoreboard_position: 0,
        })
        .collect();

    bettors.sort_by(|a, b| {
        a.total_score
            .cmp(&b.total_score)
            .then_with(|| a.bettor_name.cmp(&b.bettor_name))
    });

    for (i, bettor) in bettors.iter_mut().enumerate() {
        bettor.scoreboard_position = i;
        bettor.scoreboard_position_name = match i {
            0 => "TOP GOLFER".to_string(),
            1 => "FIRST LOSER".to_string(),
            2 => "MEH".to_string(),
            3 => "SEEN BETTER DAYS".to_string(),
            4 => "NOT A CHANCE".to_string(),
            _ => "WORST OF THE WORST".to_string(),
        };
    }

    let elapsed = chrono::Utc::now().naive_utc() - scores.last_refresh;
    ScoreData {
        bettor_struct: bettors,
        score_struct: scores.score_struct.clone(),
        last_refresh: format_time_ago_for_score_view(elapsed),
        last_refresh_source: scores.last_refresh_source.clone(),
        cache_hit,
    }
}
