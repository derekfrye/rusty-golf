use std::collections::HashMap;

use crate::controller::espn::fetch_scores_from_espn;
use crate::model::{Bettors, ScoreData, format_time_ago_for_score_view};
use rusty_golf_core::storage::Storage;

/// # Errors
///
/// Will return `Err` if the database or espn api call fails
pub async fn get_data_for_scores_page(
    event_id: i32,
    year: i32,
    use_cache: bool,
    storage: &dyn Storage,
    cache_max_age: i64,
) -> Result<ScoreData, Box<dyn std::error::Error>> {
    let active_golfers = storage.get_golfers_for_event(event_id).await?;

    let golfers_and_scores = fetch_scores_from_espn(
        active_golfers.clone(),
        year,
        event_id,
        storage,
        use_cache,
        cache_max_age,
    )
    .await?;

    let mut totals: HashMap<String, i32> = HashMap::new();
    for golfer in &golfers_and_scores.score_struct {
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

    let x = chrono::Utc::now().naive_utc() - golfers_and_scores.last_refresh;

    let total_cache = ScoreData {
        bettor_struct: bettors,
        score_struct: golfers_and_scores.score_struct,
        last_refresh: format_time_ago_for_score_view(x),
        last_refresh_source: golfers_and_scores.last_refresh_source,
    };

    Ok(total_cache)
}
