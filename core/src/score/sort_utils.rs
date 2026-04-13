use crate::model::Scores;
use std::collections::HashMap;

#[must_use]
pub fn sort_scores<S: ::std::hash::BuildHasher>(
    grouped_scores: HashMap<usize, Vec<Scores>, S>,
) -> Vec<(usize, Vec<Scores>)> {
    let mut sorted_scores: Vec<(usize, Vec<Scores>)> = grouped_scores.into_iter().collect();

    sorted_scores.sort_by_key(|(group, _)| *group); // Sort by the `group` key

    sorted_scores
}
