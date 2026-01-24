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
    golfers_override: Option<&Vec<(String, i64)>>,
    current_golfers: Option<&BTreeMap<String, Vec<String>>>,
) -> Result<Vec<GolferSelection>> {
    if golfers_override.is_none() && !has_cached_events(state)? {
        println!("no events in cache; run list_events first.");
        return Ok(Vec::new());
    }

    let bettors = ensure_bettors_selected(rl, helper_state, state)?;
    if bettors.is_empty() {
        return Ok(Vec::new());
    }

    let golfers = resolve_golfers(state, golfers_override)?;
    if golfers.is_empty() {
        return Ok(Vec::new());
    }
    let golfer_names: Vec<String> = golfers.iter().map(|(name, _)| name.clone()).collect();
    let golfer_lookup: BTreeMap<String, i64> = golfers.into_iter().collect();

    let mut selections = Vec::new();
    for bettor in bettors {
        print_current_golfers(current_golfers, &bettor);
        let entries = prompt_for_golfers(
            rl,
            helper_state,
            &bettor,
            &golfer_names,
            &golfer_lookup,
            emit_output,
        )?;
        selections.extend(entries);
    }

    Ok(selections)
}

fn ensure_bettors_selected(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    state: &mut ReplState,
) -> Result<Vec<String>> {
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
    Ok(bettors)
}

fn resolve_golfers(
    state: &mut ReplState,
    golfers_override: Option<&Vec<(String, i64)>>,
) -> Result<Vec<(String, i64)>> {
    let golfers = if let Some(golfers) = golfers_override {
        golfers.clone()
    } else {
        load_cached_golfers(state)?
    };
    if golfers.is_empty() {
        if golfers_override.is_some() {
            println!("No golfers found in KV.");
        } else {
            println!("No golfers found in cache.");
        }
    }
    Ok(golfers)
}

fn print_current_golfers(
    current_golfers: Option<&BTreeMap<String, Vec<String>>>,
    bettor: &str,
) {
    let Some(current_by_bettor) = current_golfers else {
        return;
    };
    match current_by_bettor.get(bettor) {
        Some(current) if current.is_empty() => {
            println!("Current golfers for {bettor}: (none)");
        }
        Some(current) => {
            println!("Current golfers for {bettor}: {}", current.join(", "));
        }
        None => {
            println!("Current golfers for {bettor}: (none)");
        }
    }
}

fn prompt_for_golfers(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    bettor: &str,
    golfer_names: &[String],
    golfer_lookup: &BTreeMap<String, i64>,
    emit_output: bool,
) -> Result<Vec<GolferSelection>> {
    helper_state
        .borrow_mut()
        .set_mode(ReplCompletionMode::PromptItems {
            items: golfer_names.to_vec(),
            quote_items: true,
        });
    let prompt = format!("Which golfers for {bettor}? (csv or space separated, quote-delimited) ");
    let response = prompt_for_items(rl, &prompt);
    helper_state.borrow_mut().set_mode(ReplCompletionMode::Repl);
    match response {
        Ok(selected) => {
            let mut entries = Vec::new();
            for golfer in selected {
                match golfer_lookup.get(&golfer) {
                    Some(id) => {
                        let selection = GolferSelection {
                            bettor: bettor.to_string(),
                            golfer_espn_id: *id,
                        };
                        entries.push(selection.clone());
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
            Ok(entries)
        }
        Err(ReplPromptError::Interrupted) => Ok(Vec::new()),
        Err(ReplPromptError::Invalid(err, line)) => {
            println!("{}", format_parse_error(&line, err.index));
            Ok(Vec::new())
        }
        Err(ReplPromptError::Failed(err)) => Err(err),
    }
}
