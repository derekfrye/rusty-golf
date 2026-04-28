use super::round_selection::determine_default_round;
use super::*;
use crate::model::{ScoreDisplay, StringStat};
use crate::view::score::types::BettorData;

fn tee_times(count: usize) -> Vec<StringStat> {
    (1..=count)
        .map(|idx| StringStat {
            val: format!("4/{idx:02} 8:00a"),
        })
        .collect()
}

fn line_scores(round: i32, holes: std::ops::RangeInclusive<i32>) -> Vec<LineScore> {
    holes
        .map(|hole| LineScore {
            round,
            hole,
            score: 4,
            par: 4,
            score_display: ScoreDisplay::Par,
        })
        .collect()
}

fn golfer(name: &str, tee_time_count: usize, linescores: Vec<LineScore>) -> GolferData {
    GolferData {
        golfer_name: name.to_string(),
        linescores,
        tee_times: tee_times(tee_time_count),
    }
}

fn bettors(golfers: Vec<GolferData>) -> Vec<BettorData> {
    vec![BettorData {
        bettor_name: "Chris".to_string(),
        golfers,
    }]
}

#[test]
fn keeps_current_round_selected_while_anyone_is_still_playing() {
    let bettors = bettors(vec![
        golfer("A", 3, line_scores(0, 1..=18)),
        golfer("B", 3, line_scores(0, 1..=7)),
    ]);

    assert_eq!(determine_default_round(&bettors), 1);
}

#[test]
fn advances_only_one_round_after_everyone_finishes_current_round() {
    let bettors = bettors(vec![
        golfer("A", 3, line_scores(0, 1..=18)),
        golfer("B", 3, line_scores(0, 1..=18)),
    ]);

    assert_eq!(determine_default_round(&bettors), 2);
}

#[test]
fn advances_to_round_three_only_after_round_two_is_complete() {
    let mut first_golfer_scores = line_scores(0, 1..=18);
    first_golfer_scores.extend(line_scores(1, 1..=18));
    let mut second_golfer_scores = line_scores(0, 1..=18);
    second_golfer_scores.extend(line_scores(1, 1..=18));

    let bettors = bettors(vec![
        golfer("A", 4, first_golfer_scores),
        golfer("B", 4, second_golfer_scores),
    ]);

    assert_eq!(determine_default_round(&bettors), 3);
}

#[test]
fn initial_markup_uses_selected_round_for_totals_and_buttons() {
    let bettors = bettors(vec![golfer("A", 2, line_scores(0, 1..=6))]);
    let refresh = RefreshData {
        last_refresh: "1 minute".to_string(),
        last_refresh_source: crate::model::RefreshSource::Kv,
    };

    let markup = render_line_score_tables(&bettors, &refresh).into_string();

    assert!(markup.contains("linescore-round-button selected\" data-round=\"1\""));
    assert!(markup.contains("tr class=\"linescore-total\" data-round=\"1\""));
    assert!(markup.contains("tr class=\"linescore-row\" data-round=\"1\""));
    assert!(!markup.contains("linescore-total\" data-round=\"2\""));
}
