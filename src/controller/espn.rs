use std::{collections::HashMap, vec};

use crate::model::{
    event_and_scores_already_in_db, get_scores_from_db, store_scores_in_db, take_a_char_off,
    IntStat, LineScore, PlayerJsonResponse, RefreshSource, ScoreDisplay, Scores,
    ScoresAndLastRefresh, Statistic, StringStat,
};
use chrono::DateTime;
use reqwest::Client;
// use serde::{Deserialize, Serialize};
use serde_json::Value;
// use sqlx_middleware::db;
use sql_middleware::middleware::ConfigAndPool as ConfigAndPool2;
// use tokio::{fs::File, io::AsyncWriteExt};
use tokio::sync::mpsc;

pub async fn get_json_from_espn(
    scores: &[Scores],
    year: i32,
    event_id: i32,
) -> Result<PlayerJsonResponse, reqwest::Error> {
    let client = Client::new();
    let mut player_response = PlayerJsonResponse {
        data: Vec::new(),
        eup_ids: Vec::new(),
    };

    for score in scores {
        let url = format!(
            "https://site.web.api.espn.com/apis/site/v2/sports/golf/pga/leaderboard/{}/playersummary?season={}&player={}",
            event_id,
            year,
            score.espn_id
        );

        let resp = client.get(&url).send().await?;
        // let result: Value = resp.json().await?;
        let json: HashMap<String, serde_json::Value> = resp.json().await?;

        if json.contains_key("rounds") {
            player_response.data.push(json);
            player_response.eup_ids.push(score.eup_id);
        }
    }

    Ok(player_response)
}

/// Will fetch from db if you want the cache, otherwise from ESPN.
pub async fn fetch_scores_from_espn(
    scores: Vec<Scores>,
    year: i32,
    event_id: i32,
    // db: &db::Db,
    config_and_pool: &ConfigAndPool2,
    use_cache: bool,
    cache_max_age: i64,
) -> Result<ScoresAndLastRefresh, Box<dyn std::error::Error>> {
    let are_we_using_cache: bool = match use_cache {
        true => {
            let t = event_and_scores_already_in_db(config_and_pool, event_id, cache_max_age).await;
            match t {
                Ok(true) => true,
                Ok(false) => false,
                Err(_) => false,
            }
        }
        false => false,
    };

    // get the data from espn and persist it to the database
    if !are_we_using_cache {
        let x = go_get_espn_data(scores, year, event_id).await?;
        let z = store_espn_results(&x, event_id, config_and_pool).await?;
        Ok(z)
    } else {
        // we're just retrieving the data from db
        Ok(get_scores_from_db(config_and_pool, event_id, RefreshSource::Db).await?)
    }
}

async fn store_espn_results(
    scores: &Vec<Scores>,
    // year: i32,
    event_id: i32,
    // db: &db::Db,
    config_and_pool: &ConfigAndPool2,
) -> Result<ScoresAndLastRefresh, Box<dyn std::error::Error>> {
    store_scores_in_db(config_and_pool, event_id, scores).await?;
    Ok(get_scores_from_db(config_and_pool, event_id, RefreshSource::Espn).await?)
}

async fn go_get_espn_data(
    scores: Vec<Scores>,
    year: i32,
    event_id: i32,
) -> Result<Vec<Scores>, Box<dyn std::error::Error>> {
    let num_scores = scores.len();
    let group_size = (num_scores + 3) / 4;
    let (tx, mut rx) = mpsc::channel(4);
    // Set up variables based on execution mode
    let json_responses = if cfg!(debug_assertions) {
        // In debug mode, fetch data directly without spawning tasks
        get_json_from_espn(&scores, year, event_id).await?
    } else {
        // Process in parallel using multiple tasks
        for task_index in 0..4 {
            let player_group = scores
                .iter()
                .skip(task_index * group_size)
                .take(group_size)
                .cloned()
                .collect::<Vec<_>>();

            let tx = tx.clone();

            tokio::task::spawn(async move {
                match get_json_from_espn(&player_group, year, event_id).await {
                    Ok(result) => {
                        if let Err(err) = tx.send(Some(result)).await {
                            eprintln!("Failed to send ESPN data through channel: {}", err);
                        }
                    },
                    Err(err) => {
                        eprintln!("Failed to get ESPN data: {}", err);
                        if let Err(channel_err) = tx.send(None).await {
                            eprintln!("Failed to send error notification: {}", channel_err);
                        }
                    },
                }
            });
        }

        drop(tx);
        
        // Collect results from parallel tasks
        let mut combined_response = PlayerJsonResponse {
            data: Vec::new(),
            eup_ids: Vec::new(),
        };
        
        // Process results as they arrive
        while let Some(Some(result)) = rx.recv().await {
            combined_response.data.extend(result.data);
            combined_response.eup_ids.extend(result.eup_ids);
        }
        
        combined_response
    }

    let mut golfer_scores = Vec::new();

    for (response_idx, result) in json_responses.data.iter().enumerate() {
        // Use an empty vector by default if rounds data is missing
        let rounds = result
            .get("rounds")
            .and_then(Value::as_array)
            .unwrap_or(&[]); // Use slice reference to empty array instead of creating new Vec

        let mut golfer_score = Statistic {
            eup_id: json_responses.eup_ids[response_idx],
            rounds: Vec::new(),
            round_scores: Vec::new(),
            tee_times: Vec::new(),
            holes_completed_by_round: Vec::new(),
            line_scores: Vec::new(),
            total_score: 0,
        };

        // let mut line_scores: Vec<LineScore> = vec![];
        for (i, round) in rounds.iter().enumerate() {
            // Access the line scores data with a default empty slice
            let line_scores_json = round
                .get("linescores")
                .and_then(Value::as_array)
                .unwrap_or(&[]);
            // let x = serde_json::to_string_pretty(round.get("linescores").unwrap()).unwrap();
            // let z = x.len();
            // dbg!(&line_scores);

            // let mut line_scores: Vec<LineScore> = vec![];

            for (idx, ln_score) in line_scores_json.iter().enumerate() {
                // let line_score = line_scores_json[i].as_object().unwrap();
                let par = ln_score.get("par").and_then(Value::as_i64);
                let score = ln_score.get("displayValue").and_then(Value::as_str);

                let par = par.unwrap_or(0);
                let score = score
                    .unwrap_or("")
                    .trim_start_matches('+')
                    .parse::<i64>()
                    .unwrap_or(0);
                // Use the From trait to convert score difference directly
                // Convert the score difference to i32 safely
                let score_diff = match i32::try_from(par - score) {
                    Ok(val) => val,
                    Err(_) => {
                        eprintln!("Warning: Failed to convert score difference to i32");
                        0 // Default to par if conversion fails
                    }
                };
                let score_display = ScoreDisplay::from(score_diff);

                let line_score_tmp = LineScore {
                    hole: (idx as i32) + 1,
                    score: score as i32,
                    par: par as i32,
                    // last_refresh_date: chrono::Utc::now().to_rfc3339(),
                    score_display,
                    round: i as i32,
                };
                golfer_score.line_scores.push(line_score_tmp);
            }

            let display_value = round.get("displayValue").and_then(Value::as_str);

            let display_value = display_value.unwrap_or("");

            golfer_score.rounds.push(IntStat {
                val: i as i32,
                // last_refresh_date: chrono::Utc::now().to_rfc3339(),
            });

            // Parse the score value, defaulting to 0 if parsing fails
            let score = display_value
                .trim_start_matches('+')
                .parse::<i32>()
                .unwrap_or(0);
            golfer_score.round_scores.push(IntStat {
                val: score,
                // last_refresh_date: chrono::Utc::now().to_rfc3339(),
            });

            // expected 1985-04-12T23:20:50.52Z
            // actual: 2024-05-16T18:12Z
            // oddly, crate tokio-postgres is providing the time crate :/
            //     let format = format_description::parse("[year]-[month]-[day]T[hour]:[minute]Z").unwrap();

            //     let parsed_time = time::OffsetDateTime::parse(tee_time, &format)
            // .unwrap_or(time::OffsetDateTime::now_utc())
            // .to_offset(time::UtcOffset::from_hms(0, 0, 0).unwrap());

            let tee_time = round.get("teeTime").and_then(Value::as_str).unwrap_or("");
            // Format time string properly using format! macro instead of manual concatenation
            let mut_tee_time = if tee_time.ends_with("Z") {
                format!("{}+0000", tee_time)
            } else {
                tee_time.to_owned()
            };

            let parsed_time =
                DateTime::parse_from_str(&mut_tee_time, "%Y-%m-%dT%H:%MZ%z").unwrap_or_default();

            // Use a safe conversion to Central time timezone, with fallback to UTC if conversion fails
            let central_timezone = chrono::offset::FixedOffset::east_opt(-5 * 3600)
                .unwrap_or_else(|| chrono::offset::FixedOffset::east_opt(0).unwrap());
                
            let parsed_time_in_central = parsed_time.with_timezone(&central_timezone);

            let special_format_time =
                take_a_char_off(&parsed_time_in_central.format("%-m/%d %-I:%M%P").to_string())
                    .to_string();

            // let offset_time = crate::time::OffsetDateTime::from_unix_timestamp(parsed_time.timestamp()).unwrap();
            // let time_format = crate::time::format_description::parse("[month]/[day] [hour repr=12]:[minute][period]").unwrap();
            // let formatted_time = offset_time.format(&time_format).unwrap();

            golfer_score.tee_times.push(StringStat {
                val: special_format_time,
                // last_refresh_date: chrono::Utc::now().to_rfc3339(),
            });

            let holes_completed = golfer_score.line_scores.len();
            golfer_score.holes_completed_by_round.push(IntStat {
                val: holes_completed as i32,
                // last_refresh_date: chrono::Utc::now().to_rfc3339(),
            });
        }

        golfer_score.total_score = golfer_score.round_scores.iter().map(|s| s.val).sum();
        golfer_scores.push(golfer_score);
    }

    // golfer_scores.sort_by(|a, b| {
    //     if a.group == b.group {
    //         a.eup_id.cmp(&b.eup_id)
    //     } else {
    //         a.group.cmp(&b.group)
    //     }
    // })

    let mut golfers_and_scores = Vec::with_capacity(golfer_scores.len());
    
    for statistic in &golfer_scores {
        // Find the matching golfer or handle the error case
        let active_golfer = match scores.iter().find(|g| g.eup_id == statistic.eup_id) {
            Some(golfer) => golfer,
            None => {
                // Return early with an error if no matching golfer is found
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Failed to find golfer with eup_id {}", statistic.eup_id)
                )));
            }
        };
        
        golfers_and_scores.push(Scores {
            eup_id: statistic.eup_id,
            golfer_name: active_golfer.golfer_name.clone(),
            detailed_statistics: statistic.clone(),
            bettor_name: active_golfer.bettor_name.clone(),
            group: active_golfer.group,
            espn_id: active_golfer.espn_id,
        });
    }

    golfers_and_scores.sort_by(|a, b| {
        if a.group == b.group {
            a.eup_id.cmp(&b.eup_id)
        } else {
            a.group.cmp(&b.group)
        }
    });

    Ok(golfers_and_scores)
}
