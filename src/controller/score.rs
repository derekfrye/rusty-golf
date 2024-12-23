use crate::controller::cache::{get_or_create_cache, xya};
use crate::controller::espn::fetch_scores_from_espn;
use crate::model;

use crate::model::{Bettors, Cache, CacheMap, ScoreData, Scores, SummaryScore, SummaryScores};

use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

pub async fn get_data_for_scores_page(
    event_id: i32,
    year: i32,
    cache_map: &CacheMap,
    use_cache: bool,
    db: sqlx_middleware::db::db::Db,
) -> Result<ScoreData, Box<dyn std::error::Error>> {
    let cache = get_or_create_cache(event_id, year, cache_map.clone()).await;
    if use_cache {
        if let Ok(cache) = xya(cache) {
            return Ok(cache);
        }
    }

    // reviewed, ok now for debugging
    let aactive_golfers = model::get_golfers_from_db(&db, event_id).await;
    let active_golfers = match aactive_golfers {
        Ok(active_golfers) => active_golfers.return_result,
        Err(e) => {
            return Err(e);
        }
    };

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
        };
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

pub fn group_by_scores(scores: Vec<Scores>) -> Vec<(usize, Vec<Scores>)> {
    let mut grouped_scores: HashMap<usize, Vec<Scores>> = HashMap::new();

    for score in scores {
        grouped_scores
            .entry(score.group as usize)
            .or_default()
            .push(score);
    }

    sort_scores(grouped_scores)
}

pub fn sort_scores(grouped_scores: HashMap<usize, Vec<Scores>>) -> Vec<(usize, Vec<Scores>)> {
    let mut sorted_scores: Vec<(usize, Vec<Scores>)> = grouped_scores.into_iter().collect();

    sorted_scores.sort_by_key(|(group, _)| *group); // Sort by the `group` key

    sorted_scores
}

pub fn group_by_bettor_name_and_round(scores: &Vec<Scores>) -> SummaryScores {
    // key = bettor, value = hashmap of rounds and the corresponding score
    let mut rounds_by_bettor_storing_score_val: HashMap<String, Vec<(isize, isize)>> =
        HashMap::new();

    // Accumulate scores by bettor and round
    for score in scores {
        let bettor_name = &score.bettor_name;

        // for debug watching
        // let golfers_name = &score.golfer_name;
        // let _ = golfers_name.len();

        for (round_idx, round_score) in score.detailed_statistics.scores.iter().enumerate() {
            let a_single_bettors_scores = rounds_by_bettor_storing_score_val
                .entry(bettor_name.clone())
                .or_default();
            a_single_bettors_scores.push((round_idx.try_into().unwrap(), round_score.val as isize));

            // for debug watching
            // let golfers_namex = &score.golfer_name;
            // let _ = golfers_namex.len();
        }
    }

    let mut summary_scores = SummaryScores {
        summary_scores: Vec::new(),
    };
    let mut bettor_names: Vec<String> = Vec::new();

    // Preserves order of bettors
    for score in scores {
        let bettor_name = &score.bettor_name;
        if rounds_by_bettor_storing_score_val.contains_key(bettor_name)
            && !bettor_names.contains(bettor_name)
        {
            bettor_names.push(bettor_name.clone());
        }
    }

    // Preserves order of bettors
    // this actually just needs to sum all the scores where the rounds are 0, store that val, sum all scores where rounds are 1, store that value, etc
    for bettor_name in &bettor_names {
        if rounds_by_bettor_storing_score_val
            .contains_key(bettor_name)
        {
            let res1 = rounds_by_bettor_storing_score_val
                .get(bettor_name)
                .unwrap()
                .iter();

            let result = res1
                .fold(BTreeMap::new(), |mut acc, &(k, v)| {
                    *acc.entry(k).or_insert(0) += v;
                    acc
                })
                .into_iter()
                .collect::<Vec<(isize, isize)>>();

            let (computed_rounds, new_scores): (Vec<isize>, Vec<isize>) =
                result.iter().cloned().unzip();

            summary_scores.summary_scores.push(SummaryScore {
                bettor_name: bettor_name.clone(),
                computed_rounds,
                new_scores,
            });
        }
    }

    summary_scores
}
