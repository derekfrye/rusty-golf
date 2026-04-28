use crate::view::score::types::{BettorData, GolferData};
use std::collections::BTreeSet;

pub(super) fn determine_default_round(bettors: &[BettorData]) -> usize {
    let golfers = bettors.iter().flat_map(|bettor| bettor.golfers.iter());
    let current_round = golfers
        .clone()
        .filter_map(highest_started_round)
        .max()
        .unwrap_or(1);

    let everyone_done_with_current_round = golfers
        .filter(|golfer| golfer_has_round_available(golfer, current_round))
        .all(|golfer| golfer_completed_round(golfer, current_round));

    if everyone_done_with_current_round {
        current_round + 1
    } else {
        current_round
    }
}

fn highest_started_round(golfer: &GolferData) -> Option<usize> {
    golfer
        .linescores
        .iter()
        .filter_map(|ls| usize::try_from(ls.round + 1).ok())
        .max()
}

fn golfer_has_round_available(golfer: &GolferData, round: usize) -> bool {
    golfer.tee_times.len() >= round
        || highest_started_round(golfer).is_some_and(|started| started >= round)
}

fn golfer_completed_round(golfer: &GolferData, round: usize) -> bool {
    let round_zero_based = match i32::try_from(round) {
        Ok(value) => value - 1,
        Err(_) => return false,
    };

    let completed_holes = golfer
        .linescores
        .iter()
        .filter(|ls| ls.round == round_zero_based)
        .map(|ls| ls.hole)
        .collect::<BTreeSet<_>>();

    completed_holes.len() >= 18
}
