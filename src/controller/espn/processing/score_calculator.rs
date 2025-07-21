use crate::model::{IntStat, LineScore, ScoreDisplay};
use serde_json::Value;

#[must_use]
pub fn process_line_scores(line_scores_json: &[Value], round_index: usize) -> Vec<LineScore> {
    let mut line_scores = Vec::new();

    for (idx, ln_score) in line_scores_json.iter().enumerate() {
        let par = ln_score.get("par").and_then(Value::as_i64);
        let score = ln_score.get("displayValue").and_then(Value::as_str);

        let par = par.unwrap_or(0);
        let score = score
            .unwrap_or("")
            .trim_start_matches('+')
            .parse::<i64>()
            .unwrap_or(0);

        let score_diff = if let Ok(val) = i32::try_from(par - score) { val } else {
            eprintln!("Warning: Failed to convert score difference to i32");
            0
        };
        let score_display = ScoreDisplay::from(score_diff);

        let line_score_tmp = LineScore {
            hole: i32::try_from(idx).unwrap_or(0) + 1,
            score: i32::try_from(score).unwrap_or(0),
            par: i32::try_from(par).unwrap_or(0),
            score_display,
            round: i32::try_from(round_index).unwrap_or(0),
        };
        line_scores.push(line_score_tmp);
    }

    line_scores
}

#[must_use]
pub fn process_round_score(display_value: &str, _round_index: usize) -> IntStat {
    let score = display_value
        .trim_start_matches('+')
        .parse::<i32>()
        .unwrap_or(0);
    IntStat { val: score }
}

#[must_use]
pub fn calculate_total_score(round_scores: &[IntStat]) -> i32 {
    round_scores.iter().map(|s| s.val).sum()
}
