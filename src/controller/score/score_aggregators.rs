use std::collections::{BTreeMap, HashMap};

use super::sort_utils::sort_scores;
use crate::model::{
    AllBettorScoresByRound, BettorScoreByRound, DetailedScore, Scores, SummaryDetailedScores,
};

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

pub fn group_by_bettor_name_and_round(scores: &[Scores]) -> AllBettorScoresByRound {
    let mut rounds_by_bettor_storing_score_val: HashMap<String, Vec<(isize, isize)>> =
        HashMap::new();

    for score in scores {
        let bettor_name = &score.bettor_name;

        for (round_idx, round_score) in score.detailed_statistics.round_scores.iter().enumerate() {
            let a_single_bettors_scores = rounds_by_bettor_storing_score_val
                .entry(bettor_name.to_string())
                .or_default();
            let round_idx_isize = match isize::try_from(round_idx) {
                Ok(val) => val,
                Err(_) => {
                    eprintln!("Warning: Failed to convert round index {round_idx} to isize");
                    0
                }
            };
            a_single_bettors_scores.push((round_idx_isize, round_score.val as isize));
        }
    }

    let mut summary_scores = AllBettorScoresByRound {
        summary_scores: Vec::new(),
    };
    let mut bettor_names: Vec<String> = Vec::new();

    for score in scores {
        let bettor_name = &score.bettor_name;
        if rounds_by_bettor_storing_score_val.contains_key(bettor_name)
            && !bettor_names.contains(bettor_name)
        {
            bettor_names.push(bettor_name.clone());
        }
    }

    for bettor_name in &bettor_names {
        if rounds_by_bettor_storing_score_val.contains_key(bettor_name) {
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

            summary_scores.summary_scores.push(BettorScoreByRound {
                bettor_name: bettor_name.clone(),
                computed_rounds,
                scores_aggregated_by_golf_grp_by_rd: new_scores,
            });
        }
    }

    summary_scores
}

pub fn group_by_bettor_golfer_round(scores: &Vec<Scores>) -> SummaryDetailedScores {
    let mut scores_map: HashMap<String, HashMap<String, BTreeMap<i32, i32>>> = HashMap::new();

    let mut espn_id_map: HashMap<(String, String), i64> = HashMap::new();

    let mut bettor_order: Vec<String> = Vec::new();
    let mut golfer_order_map: HashMap<String, Vec<String>> = HashMap::new();

    for score in scores {
        let bettor_name = &score.bettor_name;
        let golfer_name = &score.golfer_name;

        if !bettor_order.contains(bettor_name) {
            bettor_order.push(bettor_name.clone());
        }

        golfer_order_map
            .entry(bettor_name.clone())
            .or_default()
            .push(golfer_name.clone());

        espn_id_map.insert((bettor_name.clone(), golfer_name.clone()), score.espn_id);

        for (round_idx, score) in score.detailed_statistics.round_scores.iter().enumerate() {
            let round_val = (round_idx as i32) + 1;
            let round_score = score.val;

            scores_map
                .entry(bettor_name.clone())
                .or_default()
                .entry(golfer_name.clone())
                .or_default()
                .entry(round_val)
                .and_modify(|e| {
                    *e += round_score;
                })
                .or_insert(round_score);
        }
    }

    for golfers in golfer_order_map.values_mut() {
        let mut seen = HashMap::new();
        golfers.retain(|golfer| seen.insert(golfer.clone(), ()).is_none());
    }

    let mut summary_scores = SummaryDetailedScores {
        detailed_scores: Vec::new(),
    };

    for bettor_name in bettor_order {
        if let Some(golfers_map) = scores_map.get(&bettor_name) {
            if let Some(golfers_ordered) = golfer_order_map.get(&bettor_name) {
                for golfer_name in golfers_ordered {
                    if let Some(rounds_map) = golfers_map.get(golfer_name) {
                        let mut rounds: Vec<(i32, i32)> =
                            rounds_map.iter().map(|(&k, &v)| (k, v)).collect();
                        rounds.sort_by_key(|&(round, _)| round);

                        let (round_numbers, round_scores): (Vec<i32>, Vec<i32>) =
                            rounds.iter().cloned().unzip();

                        let golfer_espn_id = espn_id_map
                            .get(&(bettor_name.clone(), golfer_name.clone()))
                            .copied()
                            .unwrap_or(0);

                        summary_scores.detailed_scores.push(DetailedScore {
                            bettor_name: bettor_name.clone(),
                            golfer_name: golfer_name.clone(),
                            golfer_espn_id,
                            rounds: round_numbers,
                            scores: round_scores,
                        });
                    }
                }
            }
        }
    }

    summary_scores
}
