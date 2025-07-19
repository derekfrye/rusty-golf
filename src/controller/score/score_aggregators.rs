use std::collections::{BTreeMap, HashMap};

use super::sort_utils::sort_scores;
use crate::model::{
    AllBettorScoresByRound, BettorScoreByRound, DetailedScore, Scores, SummaryDetailedScores,
};

#[must_use]
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

type BettorGolferMaps = (
    HashMap<String, HashMap<String, BTreeMap<i32, i32>>>,
    HashMap<(String, String), i64>,
    Vec<String>,
    HashMap<String, Vec<String>>,
);

fn build_bettor_golfer_maps(scores: &[Scores]) -> BettorGolferMaps {
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
                .and_modify(|e| *e += round_score)
                .or_insert(round_score);
        }
    }

    for golfers in golfer_order_map.values_mut() {
        let mut seen = HashMap::new();
        golfers.retain(|golfer| seen.insert(golfer.clone(), ()).is_none());
    }

    (scores_map, espn_id_map, bettor_order, golfer_order_map)
}

#[must_use]
pub fn group_by_bettor_name_and_round(scores: &[Scores]) -> AllBettorScoresByRound {
    let (scores_map, _, bettor_order, _) = build_bettor_golfer_maps(scores);
    let mut summary_scores = AllBettorScoresByRound {
        summary_scores: Vec::new(),
    };

    for bettor_name in &bettor_order {
        let mut rounds_by_bettor_storing_score_val: BTreeMap<isize, isize> = BTreeMap::new();
        if let Some(golfers_map) = scores_map.get(bettor_name) {
            for rounds_map in golfers_map.values() {
                for (&round, &score) in rounds_map {
                    *rounds_by_bettor_storing_score_val
                        .entry(round as isize - 1)
                        .or_insert(0) += score as isize;
                }
            }
        }

        let (computed_rounds, new_scores): (Vec<isize>, Vec<isize>) =
            rounds_by_bettor_storing_score_val.into_iter().unzip();

        summary_scores.summary_scores.push(BettorScoreByRound {
            bettor_name: bettor_name.clone(),
            computed_rounds,
            scores_aggregated_by_golf_grp_by_rd: new_scores,
        });
    }

    summary_scores
}

#[must_use]
pub fn group_by_bettor_golfer_round(scores: &[Scores]) -> SummaryDetailedScores {
    let (scores_map, espn_id_map, bettor_order, golfer_order_map) =
        build_bettor_golfer_maps(scores);

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
                            rounds.iter().copied().unzip();

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
