use super::{GolferSelection, ReplState};
use std::path::PathBuf;

pub(crate) fn set_golfers_by_bettor(
    state: &mut ReplState,
    selections: Vec<GolferSelection>,
) {
    state.golfers_by_bettor = Some(selections);
}

pub(crate) fn take_golfers_by_bettor(state: &mut ReplState) -> Option<Vec<GolferSelection>> {
    state.golfers_by_bettor.take()
}

pub(crate) fn output_json_path(state: &ReplState) -> Option<PathBuf> {
    state.output_json_path.clone()
}
