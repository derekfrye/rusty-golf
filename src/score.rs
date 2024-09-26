use crate::model::cache::{get_or_create_cache, xya};
use crate::model::db::get_golfers_from_db;
use crate::model::espn::fetch_scores_from_espn;
use crate::model::model::{Bettors, Cache, CacheMap, ScoreData, Scores};

use std::collections::HashMap;
use std::time::Instant;

pub async fn get_data_for_scores_page(
    event_id: i32,
    year: i32,
    cache_map: &CacheMap,
    use_cache: bool,
) -> Result<ScoreData, Box<dyn std::error::Error>> {
    let cache = get_or_create_cache(event_id, year, cache_map.clone()).await;
    if use_cache {
        if let Ok(cache) = xya(cache) {
            return Ok(cache);
        }
    }

    // reviewed, ok now for debugging
    let active_golfers = get_golfers_from_db(event_id).await?;
    let start_time = Instant::now();
    // reviewed, ok now for debugging
    let scores = fetch_scores_from_espn(active_golfers.clone(), year, event_id).await;

    // ok
    let mut golfers_and_scores: Vec<Scores> = scores
        .iter()
        .map(|score| {
            let active_golfer = active_golfers
                .iter()
                .find(|g| g.eup_id == score.eup_id)
                .unwrap();
            Scores {
                eup_id: score.eup_id,
                golfer_name: active_golfer.golfer_name.clone(),
                detailed_statistics: score.clone(),
                bettor_name: active_golfer.bettor_name.clone(),
                group: active_golfer.group,
                espn_id: active_golfer.espn_id,
            }
        })
        .collect();

    // ok
    golfers_and_scores.sort_by(|a, b| {
        if a.group == b.group {
            a.eup_id.cmp(&b.eup_id)
        } else {
            a.group.cmp(&b.group)
        }
    });

    // ok
    let mut totals: HashMap<String, i32> = HashMap::new();
    for golfer in &golfers_and_scores {
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

    bettors.sort_by(|a, b| a.total_score.cmp(&b.total_score));

    for (i, bettor) in bettors.iter_mut().enumerate() {
        bettor.scoreboard_position = i;
        bettor.scoreboard_position_name = match i {
            0 => "TOP GOLFER".to_string(),
            1 => "FIRST LOSER".to_string(),
            2 => "MEH".to_string(),
            3 => "SEEN BETTER DAYS".to_string(),
            4 => "NOT A CHANCE".to_string(),
            _ => "WORST OF THE WORST".to_string(),
        }
    }

    let time_since = start_time.elapsed();
    let minutes = time_since.as_secs() / 60;
    let seconds = time_since.as_secs() % 60;
    let time_string = format!("{}m, {}s", minutes, seconds);

    let total_cache = ScoreData {
        bettor_struct: bettors,
        score_struct: golfers_and_scores,
        last_refresh: time_string,
    };

    let key = format!("{}{}", event_id, year);
    let mut cache = cache_map.write().await;
    cache.insert(
        key,
        Cache {
            data: Some(total_cache.clone()),
            cached_time: chrono::Utc::now().to_rfc3339(),
        },
    );

    Ok(total_cache)
}
