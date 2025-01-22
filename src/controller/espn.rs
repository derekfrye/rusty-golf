use std::{collections::HashMap, vec};

use crate::model::{
    IntStat, LineScore, PlayerJsonResponse, ResultStatus, ScoreDsiplay, Scores, Statistic,
    StringStat,
};
use chrono::DateTime;
use reqwest::Client;
// use serde::{Deserialize, Serialize};
use serde_json::Value;
// use tokio::{fs::File, io::AsyncWriteExt};
use tokio::sync::mpsc;

pub async fn get_json_from_espn(
    scores: Vec<Scores>,
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
) -> Vec<Statistic> {
    let num_scores = scores.len();
    let group_size = (num_scores + 3) / 4;
    let (tx, mut rx) = mpsc::channel(4);
    let mut result: PlayerJsonResponse = PlayerJsonResponse {
        data: Vec::new(),
        eup_ids: Vec::new(),
    };

    if cfg!(debug_assertions) {
        result = get_json_from_espn(scores, year, event_id).await.unwrap();
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
                match get_json_from_espn(group, year, event_id).await {
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

        for (i, round) in rounds.iter().enumerate() {
            let display_value = round
                .get("displayValue")
                .and_then(Value::as_str)
                .unwrap_or("");
            let line_scores_tmp = round.get("linescores").and_then(Value::as_array);
            let line_scores = line_scores_tmp.unwrap_or(&vv);
            // dbg!(&line_scores);

            let mut line_scoress: Vec<LineScore> = vec![];
            for (idx, line_score) in line_scores.iter().enumerate() {
                let par = line_score.get("par").and_then(Value::as_i64).unwrap_or(0);
                let score = line_score.get("value").and_then(Value::as_i64).unwrap_or(0);
                let score_display = ScoreDsiplay::from_i32((par - score).try_into().unwrap());
                let line_score = LineScore {
                    hole: idx as i32,
                    score: score as i32,
                    par: par as i32,
                    success: ResultStatus::Success,
                    // last_refresh_date: chrono::Utc::now().to_rfc3339(),
                    score_display,
                };
                line_scoress.push(line_score);
            }

            let success = if !display_value.is_empty() {
                ResultStatus::Success
            } else {
                ResultStatus::NoData
            };

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

            let holes_completed = line_scores_tmp.map(|ls| ls.len()).unwrap_or(0);
            golfer_score.holes_completed_by_round.push(IntStat {
                val: holes_completed as i32,
                success: ResultStatus::Success,
                // last_refresh_date: chrono::Utc::now().to_rfc3339(),
            });
        }

        golfer_score.total_score = golfer_score.round_scores.iter().map(|s| s.val).sum();
        golfer_scores.push(golfer_score);
    }

    golfer_scores
}
