use std::{collections::HashMap, vec};

use crate::model::{
    get_scores_from_db, store_scores_in_db, IntStat, LineScore, PlayerJsonResponse, ResultStatus,
    ScoreDisplay, Scores, Statistic, StringStat,
};
use chrono::DateTime;
use reqwest::Client;
// use serde::{Deserialize, Serialize};
use serde_json::Value;
// use sqlx_middleware::db;
use sqlx_middleware::middleware::ConfigAndPool as ConfigAndPool2;
// use tokio::{fs::File, io::AsyncWriteExt};
use tokio::sync::mpsc;

pub async fn get_json_from_espn(
    scores: &Vec<Scores>,
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
            event_id, year, score.espn_id
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

pub async fn fetch_scores_from_espn(
    scores: Vec<Scores>,
    year: i32,
    event_id: i32,
    // db: &db::Db,
    config_and_pool: &ConfigAndPool2,
) -> Result<Vec<Scores>, Box<dyn std::error::Error>> {
    let x = go_get_espn_data(scores, year, event_id).await?;
    Ok(store_espn_results(&x, event_id, config_and_pool).await?)
}

async fn store_espn_results(
    scores: &Vec<Scores>,
    // year: i32,
    event_id: i32,
    // db: &db::Db,
    config_and_pool: &ConfigAndPool2,
) -> Result<Vec<Scores>, Box<dyn std::error::Error>> {
    store_scores_in_db(config_and_pool, event_id, scores).await?;
    Ok(get_scores_from_db(config_and_pool, event_id).await?)
}

async fn go_get_espn_data(
    scores: Vec<Scores>,
    year: i32,
    event_id: i32,
) -> Result<Vec<Scores>, Box<dyn std::error::Error>> {
    let num_scores = scores.len();
    let group_size = (num_scores + 3) / 4;
    let (tx, mut rx) = mpsc::channel(4);
    let mut result: PlayerJsonResponse = PlayerJsonResponse {
        data: Vec::new(),
        eup_ids: Vec::new(),
    };

    if cfg!(debug_assertions) {
        result = get_json_from_espn(&scores, year, event_id).await.unwrap();
    } else {
        // four threads
        for i in 0..4 {
            let group = scores
                .iter()
                .skip(i * group_size)
                .take(group_size)
                .cloned()
                .collect::<Vec<_>>();

            let tx = tx.clone();
            // let state = self.clone();

            tokio::task::spawn(async move {
                match get_json_from_espn(&group, year, event_id).await {
                    Ok(result) => tx.send(Some(result)).await.unwrap(),
                    Err(_) => tx.send(None).await.unwrap(),
                }
            });
        }

        drop(tx);
    }

    let mut json_responses = PlayerJsonResponse {
        data: Vec::new(),
        eup_ids: Vec::new(),
    };

    if cfg!(debug_assertions) {
        json_responses.data.extend(result.data);
        json_responses.eup_ids.extend(result.eup_ids);
    } else {
        while let Some(Some(result)) = rx.recv().await {
            json_responses.data.extend(result.data);
            json_responses.eup_ids.extend(result.eup_ids);
        }
    }

    let mut golfer_scores = Vec::new();

    for (idx, result) in json_responses.data.iter().enumerate() {
        // let x = serde_json::to_string_pretty(&result).unwrap();
        // //save to a file
        // if idx == 0 {
        //     let mut file = File::create("tests/espn.json_responses.json").await.unwrap();
        //     file.write_all(x.as_bytes()).await.unwrap();
        // }
        let rounds_temp = result.get("rounds").and_then(Value::as_array);
        let vv = vec![];
        let rounds = rounds_temp.unwrap_or(&vv);

        let mut golfer_score = Statistic {
            eup_id: json_responses.eup_ids[idx],
            rounds: Vec::new(),
            round_scores: Vec::new(),
            tee_times: Vec::new(),
            holes_completed_by_round: Vec::new(),
            line_scores: Vec::new(),
            success_fail: ResultStatus::NoData,
            total_score: 0,
        };

        // let mut line_scores: Vec<LineScore> = vec![];
        for (i, round) in rounds.iter().enumerate() {
            let line_scores_tmp = round.get("linescores").and_then(Value::as_array);
            let line_scores_json = line_scores_tmp.unwrap_or(&vv);
            // let x = serde_json::to_string_pretty(round.get("linescores").unwrap()).unwrap();
            // let z = x.len();
            // dbg!(&line_scores);

            // let mut line_scores: Vec<LineScore> = vec![];

            for (idx, ln_score) in line_scores_json.iter().enumerate() {
                // let line_score = line_scores_json[i].as_object().unwrap();
                let par = ln_score.get("par").and_then(Value::as_i64);
                let score = ln_score.get("displayValue").and_then(Value::as_str);

                let success = if par.is_none() || score.is_none() {
                    ResultStatus::NoData
                } else {
                    ResultStatus::Success
                };

                let par = par.unwrap_or(0);
                let score = score
                    .unwrap_or("")
                    .trim_start_matches('+')
                    .parse::<i64>()
                    .unwrap_or(0);
                let score_display = ScoreDisplay::from_i32((par - score).try_into().unwrap());

                let line_score_tmp = LineScore {
                    hole: (idx as i32) + 1,
                    score: score as i32,
                    par: par as i32,
                    success,
                    // last_refresh_date: chrono::Utc::now().to_rfc3339(),
                    score_display,
                    round: i as i32,
                };
                golfer_score.line_scores.push(line_score_tmp);
            }

            let display_value = round.get("displayValue").and_then(Value::as_str);

            let success = if display_value.is_none() || !display_value.unwrap_or("").is_empty() {
                ResultStatus::Success
            } else {
                ResultStatus::NoData
            };
            let display_value = display_value.unwrap_or("");

            golfer_score.rounds.push(IntStat {
                val: i as i32,
                success,
                // last_refresh_date: chrono::Utc::now().to_rfc3339(),
            });

            let score = display_value
                .trim_start_matches('+')
                .parse::<i32>()
                .unwrap_or(0);
            golfer_score.round_scores.push(IntStat {
                val: score,
                success: ResultStatus::Success,
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
            let mut mut_tee_time = tee_time.to_owned();
            if mut_tee_time.ends_with("Z") {
                mut_tee_time.push_str("+0000");
                // tee_time = (tee_time.to_owned() + "+0000").as_str();
            }

            let parsed_time =
                DateTime::parse_from_str(&mut_tee_time, "%Y-%m-%dT%H:%MZ%z").unwrap_or_default();

            let parsed_time_in_central = parsed_time
                .with_timezone(&chrono::offset::FixedOffset::east_opt(-5 * 3600).unwrap());

            let special_format_time = parsed_time_in_central.format("%-m/%d %-I:%M%P").to_string();

            // let offset_time = crate::time::OffsetDateTime::from_unix_timestamp(parsed_time.timestamp()).unwrap();
            // let time_format = crate::time::format_description::parse("[month]/[day] [hour repr=12]:[minute][period]").unwrap();
            // let formatted_time = offset_time.format(&time_format).unwrap();

            golfer_score.tee_times.push(StringStat {
                val: special_format_time,
                success: ResultStatus::Success,
                // last_refresh_date: chrono::Utc::now().to_rfc3339(),
            });

            let holes_completed = golfer_score.line_scores.len();
            golfer_score.holes_completed_by_round.push(IntStat {
                val: holes_completed as i32,
                success: ResultStatus::Success,
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

    let mut golfers_and_scores: Vec<Scores> = golfer_scores
        .iter()
        .map(|statistic| {
            let active_golfer = &scores
                .iter()
                .find(|g| g.eup_id == statistic.eup_id)
                .unwrap();
            Scores {
                eup_id: statistic.eup_id,
                golfer_name: active_golfer.golfer_name.clone(),
                detailed_statistics: statistic.clone(),
                bettor_name: active_golfer.bettor_name.clone(),
                group: active_golfer.group,
                espn_id: active_golfer.espn_id,
            }
        })
        .collect();

    golfers_and_scores.sort_by(|a, b| {
        if a.group == b.group {
            a.eup_id.cmp(&b.eup_id)
        } else {
            a.group.cmp(&b.group)
        }
    });

    Ok(golfers_and_scores)
}
