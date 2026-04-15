use super::bettors::{handle_pick_bettors, prompt_for_bettors};
use super::golfers::select_golfers_by_bettor;
use super::setup::{ensure_output_path, print_events};
use crate::repl::helper::{ReplCompletionMode, ReplHelper, ReplHelperState};
use crate::repl::parse::format_parse_error;
use crate::repl::payload::write_event_payload;
use crate::repl::prompt::{ReplPromptError, prompt_for_items};
use crate::repl::state::{
    EventListMode, ReplState, bettors_selection_exists, ensure_list_events, load_bettors_selection,
    load_current_golfers_by_bettor, load_kv_bettors, load_kv_golfers_list,
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
    let events = ensure_list_events(state, EventListMode::EnsureAll, true)?;
    print_events(&events);
    if events.is_empty() {
        return Ok(());
    }

    let Some((event_id, event_id_raw)) = select_update_event(rl, helper_state, &events)? else {
        return Ok(());
    };

    let Some(bettors) = select_bettors(rl, helper_state, state, event_id)? else {
        return Ok(());
    };

    let current_golfers = load_current_golfers(state, event_id);
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
    let event_name = event_name_for_id(&events, event_id, &event_id_raw);
    let golfers = load_golfers_for_event(state, event_id)?;
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

fn select_update_event(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    events: &[(String, String)],
) -> Result<Option<(i64, String)>> {
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
        return Ok(None);
    };
    let event_id: i64 = if let Ok(id) = event_id_raw.parse() {
        id
    } else {
        println!("Invalid event id: {event_id_raw}");
        return Ok(None);
    };
    Ok(Some((event_id, event_id_raw)))
}

fn select_bettors(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    state: &mut ReplState,
    event_id: i64,
) -> Result<Option<Vec<String>>> {
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
        return Ok(None);
    }
    let bettors = load_bettors_selection(state)?;
    if bettors.is_empty() {
        println!("No bettors selected.");
        return Ok(None);
    }
    Ok(Some(bettors))
}

fn load_current_golfers(
    state: &ReplState,
    event_id: i64,
) -> Option<std::collections::BTreeMap<String, Vec<String>>> {
    if state.kv_access.is_none() {
        println!("KV access not configured; skipping current golfer lookup.");
        return None;
    }
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
}

fn event_name_for_id(events: &[(String, String)], event_id: i64, raw_id: &str) -> String {
    let event_id_str = event_id.to_string();
    events
        .iter()
        .find(|(id, _)| id == &event_id_str)
        .map_or_else(|| raw_id.to_string(), |(_, name)| name.clone())
}

fn load_golfers_for_event(state: &mut ReplState, event_id: i64) -> Result<Vec<(String, i64)>> {
    let kv_golfers = match load_kv_golfers_list(state, event_id) {
        Ok(Some(golfers)) => Some(golfers),
        Ok(None) => None,
        Err(err) => {
            println!("Failed to load golfers from KV: {err}");
            None
        }
    };
    Ok(if let Some(kv_golfers) = kv_golfers {
        kv_golfers
    } else {
        crate::repl::state::load_cached_golfers(state)?
    })
}
