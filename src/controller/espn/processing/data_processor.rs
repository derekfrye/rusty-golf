use super::score_calculator::{calculate_total_score, process_line_scores, process_round_score};
use super::time_processor::process_tee_time;
use crate::model::{IntStat, PlayerJsonResponse, Scores, Statistic};
use serde_json::Value;

/// # Errors
///
/// Will return `Err` if the json processing fails
pub fn process_json_to_statistics(
    json_responses: &PlayerJsonResponse,
) -> Result<Vec<Statistic>, Box<dyn std::error::Error>> {
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

            let line_scores = process_line_scores(line_scores_json, i);
            golfer_score.line_scores.extend(line_scores);

            let display_value = round.get("displayValue").and_then(Value::as_str);
            let display_value = display_value.unwrap_or("");

            golfer_score.rounds.push(IntStat { val: i32::try_from(i).unwrap_or(0) });
            golfer_score
                .round_scores
                .push(process_round_score(display_value, i));

            let tee_time = round.get("teeTime").and_then(Value::as_str).unwrap_or("");
            if let Some(processed_tee_time) = process_tee_time(tee_time) {
                golfer_score.tee_times.push(processed_tee_time);
            }

            let holes_completed = golfer_score.line_scores.len();
            golfer_score.holes_completed_by_round.push(IntStat {
                val: i32::try_from(holes_completed).unwrap_or(0),
            });
        }

        golfer_score.total_score = calculate_total_score(&golfer_score.round_scores);
        golfer_scores.push(golfer_score);
    }

    Ok(golfer_scores)
}

/// # Errors
///
/// Will return `Err` if there is a mismatch between statistics and scores
pub fn merge_statistics_with_scores(
    statistics: &[Statistic],
    scores: &[Scores],
) -> Result<Vec<Scores>, Box<dyn std::error::Error>> {
    let result: Result<Vec<_>, _> = statistics
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
                .map(|active_golfer| Scores {
                    eup_id: statistic.eup_id,
                    golfer_name: active_golfer.golfer_name.clone(),
                    detailed_statistics: statistic.clone(),
                    bettor_name: active_golfer.bettor_name.clone(),
                    group: active_golfer.group,
                    espn_id: active_golfer.espn_id,
                    score_view_step_factor: active_golfer.score_view_step_factor,
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
