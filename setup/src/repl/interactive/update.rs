use super::bettors::{handle_pick_bettors, prompt_for_bettors};
use super::golfers::select_golfers_by_bettor;
use super::setup::{ensure_output_path, print_events};
use crate::repl::helper::{ReplCompletionMode, ReplHelper, ReplHelperState};
use crate::repl::parse::format_parse_error;
use crate::repl::payload::write_event_payload;
use crate::repl::prompt::{ReplPromptError, prompt_for_items};
use crate::repl::state::{
    ReplState, bettors_selection_exists, ensure_list_events, load_bettors_selection,
    load_current_golfers_by_bettor, load_kv_bettors,
    load_kv_golfers_list,
};
use anyhow::Result;
use rustyline::Editor;
use rustyline::history::DefaultHistory;
use std::cell::RefCell;
use std::rc::Rc;

pub(super) fn run_update_event(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    state: &mut ReplState,
) -> Result<()> {
    let events = ensure_list_events(state, false, true)?;
    print_events(&events);
    if events.is_empty() {
        return Ok(());
    }

    let event_ids: Vec<String> = events.iter().map(|(id, _)| id.clone()).collect();
    helper_state
        .borrow_mut()
        .set_mode(ReplCompletionMode::PromptItems {
            items: event_ids,
            quote_items: false,
        });
    let response = prompt_for_items(rl, "Update which event? ");
    helper_state.borrow_mut().set_mode(ReplCompletionMode::Repl);
    let selected_event = match response {
        Ok(mut selected) => selected.drain(..).next(),
        Err(ReplPromptError::Interrupted) => None,
        Err(ReplPromptError::Invalid(err, line)) => {
            println!("{}", format_parse_error(&line, err.index));
            None
        }
        Err(ReplPromptError::Failed(err)) => return Err(err),
    };
    let Some(event_id_raw) = selected_event else {
        return Ok(());
    };
    let event_id: i64 = if let Ok(id) = event_id_raw.parse() {
        id
    } else {
        println!("Invalid event id: {event_id_raw}");
        return Ok(());
    };

    let kv_bettors = match load_kv_bettors(state, event_id) {
        Ok(Some(bettors)) => Some(bettors),
        Ok(None) => None,
        Err(err) => {
            println!("Failed to load bettors from KV: {err}");
            None
        }
    };
    if let Some(kv_bettors) = kv_bettors.clone() {
        if kv_bettors.is_empty() {
            println!("No bettors found.");
        } else {
            for bettor in &kv_bettors {
                println!("{bettor}");
            }
        }
        prompt_for_bettors(rl, helper_state, state, kv_bettors)?;
    } else {
        handle_pick_bettors(rl, helper_state, state)?;
    }
    if !bettors_selection_exists(state) {
        println!("No bettors selected.");
        return Ok(());
    }
    let bettors = load_bettors_selection(state)?;
    if bettors.is_empty() {
        println!("No bettors selected.");
        return Ok(());
    }

    let current_golfers = if state.kv_access.is_none() {
        println!("KV access not configured; skipping current golfer lookup.");
        None
    } else {
        match load_current_golfers_by_bettor(state, event_id) {
            Ok(Some(current)) => Some(current),
            Ok(None) => {
                println!("No current golfers found in KV for event {event_id}.");
                None
            }
            Err(err) => {
                println!("Failed to load current golfers from KV: {err}");
                None
            }
        }
    };

    let selections = select_golfers_by_bettor(
        rl,
        helper_state,
        state,
        false,
        None,
        current_golfers.as_ref(),
    )?;
    if selections.is_empty() {
        return Ok(());
    }

    let output_path = ensure_output_path(rl, helper_state, state)?;
    let event_name = events
        .iter()
        .find(|(id, _)| *id == event_id_raw)
        .map_or_else(|| event_id_raw.clone(), |(_, name)| name.clone());
    let kv_golfers = match load_kv_golfers_list(state, event_id) {
        Ok(Some(golfers)) => Some(golfers),
        Ok(None) => None,
        Err(err) => {
            println!("Failed to load golfers from KV: {err}");
            None
        }
    };
    let golfers = if let Some(kv_golfers) = kv_golfers.as_ref() {
        kv_golfers.clone()
    } else {
        crate::repl::state::load_cached_golfers(state)?
    };
    write_event_payload(
        state,
        &output_path,
        event_id,
        &event_name,
        &golfers,
        &bettors,
        &selections,
    )
}
