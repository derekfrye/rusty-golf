use serde_json::Value;
use chrono::DateTime;

use crate::model::{
    IntStat, LineScore, PlayerJsonResponse, ScoreDisplay, Scores, 
    Statistic, StringStat, take_a_char_off
};
use crate::controller::espn::client::get_json_from_espn;

pub async fn go_get_espn_data(
    scores: Vec<Scores>,
    year: i32,
    event_id: i32,
) -> Result<Vec<Scores>, Box<dyn std::error::Error>> {
    let json_responses = if cfg!(debug_assertions) {
        get_json_from_espn(&scores, year, event_id).await?
    } else {
        use futures::future::join_all;

        let num_scores = scores.len();
        let group_size = num_scores.div_ceil(4);
        let mut futures = Vec::with_capacity(4);

        for task_index in 0..4 {
            let player_group = scores
                .iter()
                .skip(task_index * group_size)
                .take(group_size)
                .cloned()
                .collect::<Vec<_>>();

            if player_group.is_empty() {
                continue;
            }

            let player_group_clone = player_group.clone();
            let future = tokio::task::spawn(async move {
                match get_json_from_espn(&player_group_clone, year, event_id).await {
                    Ok(response) => Some(response),
                    Err(err) => {
                        eprintln!("Failed to get ESPN data: {err}");
                        None
                    }
                }
            });

            futures.push(future);
        }

        let results = join_all(futures).await;

        let mut combined_response = PlayerJsonResponse {
            data: Vec::new(),
            eup_ids: Vec::new(),
        };

        for response in results.into_iter().flatten().flatten() {
            combined_response.data.extend(response.data);
            combined_response.eup_ids.extend(response.eup_ids);
        }

        combined_response
    };

    let mut golfer_scores = Vec::new();
    let empty_vec: Vec<Value> = Vec::new();

    for (response_idx, result) in json_responses.data.iter().enumerate() {
        let rounds = result
            .get("rounds")
            .and_then(Value::as_array)
            .unwrap_or(&empty_vec);

        let mut golfer_score = Statistic {
            eup_id: json_responses.eup_ids[response_idx],
            rounds: Vec::new(),
            round_scores: Vec::new(),
            tee_times: Vec::new(),
            holes_completed_by_round: Vec::new(),
            line_scores: Vec::new(),
            total_score: 0,
        };

        for (i, round) in rounds.iter().enumerate() {
            let line_scores_json = round
                .get("linescores")
                .and_then(Value::as_array)
                .unwrap_or(&empty_vec);

            for (idx, ln_score) in line_scores_json.iter().enumerate() {
                let par = ln_score.get("par").and_then(Value::as_i64);
                let score = ln_score.get("displayValue").and_then(Value::as_str);

                let par = par.unwrap_or(0);
                let score = score
                    .unwrap_or("")
                    .trim_start_matches('+')
                    .parse::<i64>()
                    .unwrap_or(0);

                let score_diff = match i32::try_from(par - score) {
                    Ok(val) => val,
                    Err(_) => {
                        eprintln!("Warning: Failed to convert score difference to i32");
                        0
                    }
                };
                let score_display = ScoreDisplay::from(score_diff);

                let line_score_tmp = LineScore {
                    hole: (idx as i32) + 1,
                    score: score as i32,
                    par: par as i32,
                    score_display,
                    round: i as i32,
                };
                golfer_score.line_scores.push(line_score_tmp);
            }

            let display_value = round.get("displayValue").and_then(Value::as_str);
            let display_value = display_value.unwrap_or("");

            golfer_score.rounds.push(IntStat {
                val: i as i32,
            });

            let score = display_value
                .trim_start_matches('+')
                .parse::<i32>()
                .unwrap_or(0);
            golfer_score.round_scores.push(IntStat {
                val: score,
            });

            let tee_time = round.get("teeTime").and_then(Value::as_str).unwrap_or("");
            let mut_tee_time = if tee_time.ends_with("Z") {
                format!("{tee_time}+0000")
            } else {
                tee_time.to_owned()
            };

            let mut failed_to_parse = false;
            let parsed_time = match DateTime::parse_from_str(&mut_tee_time, "%Y-%m-%dT%H:%MZ%z") {
                Ok(dt) => dt,
                Err(_e) => {
                    failed_to_parse = true;
                    DateTime::parse_from_rfc3339("2000-01-01T00:00:00+00:00")
                        .expect("Hardcoded fallback date should always be valid")
                }
            };

            let central_timezone =
                chrono::offset::FixedOffset::east_opt(-5 * 3600).unwrap_or_else(|| {
                    chrono::offset::FixedOffset::east_opt(0)
                        .expect("UTC timezone offset is always valid")
                });

            let parsed_time_in_central = parsed_time.with_timezone(&central_timezone);

            let special_format_time =
                take_a_char_off(&parsed_time_in_central.format("%-m/%d %-I:%M%P").to_string())
                    .to_string();

            if !failed_to_parse {
                golfer_score.tee_times.push(StringStat {
                    val: special_format_time,
                });
            }

            let holes_completed = golfer_score.line_scores.len();
            golfer_score.holes_completed_by_round.push(IntStat {
                val: holes_completed as i32,
            });
        }

        golfer_score.total_score = golfer_score.round_scores.iter().map(|s| s.val).sum();
        golfer_scores.push(golfer_score);
    }

    let result: Result<Vec<_>, _> = golfer_scores
        .iter()
        .map(|statistic| {
            scores
                .iter()
                .find(|g| g.eup_id == statistic.eup_id)
                .ok_or_else(|| {
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!(
                            "Failed to find golfer with eup_id {} in scores data",
                            statistic.eup_id
                        ),
                    )) as Box<dyn std::error::Error>
                })
                .map(|active_golfer| {
                    Scores {
                        eup_id: statistic.eup_id,
                        golfer_name: active_golfer.golfer_name.clone(),
                        detailed_statistics: statistic.clone(),
                        bettor_name: active_golfer.bettor_name.clone(),
                        group: active_golfer.group,
                        espn_id: active_golfer.espn_id,
                        score_view_step_factor: active_golfer.score_view_step_factor,
                    }
                })
        })
        .collect::<Result<Vec<Scores>, Box<dyn std::error::Error>>>();

    let mut golfers_and_scores = result?;

    golfers_and_scores.sort_by(|a, b| {
        if a.group == b.group {
            a.eup_id.cmp(&b.eup_id)
        } else {
            a.group.cmp(&b.group)
        }
    });

    Ok(golfers_and_scores)
}