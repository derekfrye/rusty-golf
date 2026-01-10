use super::bettors::handle_pick_bettors;
use crate::repl::helper::{ReplCompletionMode, ReplHelper, ReplHelperState};
use crate::repl::parse::format_parse_error;
use crate::repl::prompt::{ReplPromptError, prompt_for_items};
use crate::repl::state::{
    GolferSelection, ReplState, bettors_selection_exists, has_cached_events,
    load_bettors_selection, load_cached_golfers,
};
use anyhow::Result;
use rustyline::Editor;
use rustyline::history::DefaultHistory;
use serde_json::Value;
use serde_json::json;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

pub(super) fn select_golfers_by_bettor(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    state: &mut ReplState,
    emit_output: bool,
) -> Result<Vec<GolferSelection>> {
    if !has_cached_events(state)? {
        println!("no events in cache; run list_events first.");
        return Ok(Vec::new());
    }

    if !bettors_selection_exists(state) {
        handle_pick_bettors(rl, helper_state, state)?;
    }

    if !bettors_selection_exists(state) {
        println!("No bettors selected.");
        return Ok(Vec::new());
    }

    let bettors = load_bettors_selection(state)?;
    if bettors.is_empty() {
        println!("No bettors selected.");
        return Ok(Vec::new());
    }

    let golfers = load_cached_golfers(state)?;
    if golfers.is_empty() {
        println!("No golfers found in cache.");
        return Ok(Vec::new());
    }
    let golfer_names: Vec<String> = golfers.iter().map(|(name, _)| name.clone()).collect();
    let golfer_lookup: BTreeMap<String, i64> = golfers.into_iter().collect();

    let mut selections = Vec::new();
    for bettor in bettors {
        helper_state
            .borrow_mut()
            .set_mode(ReplCompletionMode::PromptItems {
                items: golfer_names.clone(),
                quote_items: true,
            });
        let prompt =
            format!("Which golfers for {bettor}? (csv or space separated, quote-delimited) ");
        let response = prompt_for_items(rl, &prompt);
        helper_state.borrow_mut().set_mode(ReplCompletionMode::Repl);
        match response {
            Ok(selected) => {
                let mut entries = Vec::new();
                for golfer in selected {
                    match golfer_lookup.get(&golfer) {
                        Some(id) => {
                            let selection = GolferSelection {
                                bettor: bettor.clone(),
                                golfer_espn_id: *id,
                            };
                            entries.push(selection.clone());
                            selections.push(selection);
                        }
                        None => {
                            println!("Unknown golfer: {golfer}");
                        }
                    }
                }
                if emit_output {
                    let payload: Vec<Value> = entries
                        .iter()
                        .map(|entry| {
                            json!({
                                "bettor": entry.bettor,
                                "golfer_espn_id": entry.golfer_espn_id,
                            })
                        })
                        .collect();
                    println!("{}", serde_json::to_string(&payload)?);
                }
            }
            Err(ReplPromptError::Interrupted) => {}
            Err(ReplPromptError::Invalid(err, line)) => {
                println!("{}", format_parse_error(&line, err.index));
            }
            Err(ReplPromptError::Failed(err)) => return Err(err),
        }
    }

    Ok(selections)
}
