use super::bettors::{handle_pick_bettors, prompt_for_bettors};
use super::golfers::select_golfers_by_bettor;
use crate::repl::helper::{ReplCompletionMode, ReplHelper, ReplHelperState};
use crate::repl::parse::format_parse_error;
use crate::repl::prompt::{ReplPromptError, prompt_for_items};
use crate::repl::state::{
    ReplState, bettors_selection_exists, load_kv_bettors, load_kv_golfers_list,
    take_golfers_by_bettor,
};
use anyhow::Result;
use rustyline::Editor;
use rustyline::history::DefaultHistory;
use std::cell::RefCell;
use std::rc::Rc;

type KvSeedData = (Option<Vec<String>>, Option<Vec<(String, i64)>>);

pub(super) fn select_setup_event(
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
    let response = prompt_for_items(rl, "Setup which event? ");
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

pub(super) fn load_kv_seed_data(state: &mut ReplState, event_id: i64) -> KvSeedData {
    let kv_bettors = match load_kv_bettors(state, event_id) {
        Ok(Some(bettors)) => Some(bettors),
        Ok(None) => None,
        Err(err) => {
            println!("Failed to load bettors from KV: {err}");
            None
        }
    };
    let kv_golfers = match load_kv_golfers_list(state, event_id) {
        Ok(Some(golfers)) => Some(golfers),
        Ok(None) => None,
        Err(err) => {
            println!("Failed to load golfers from KV: {err}");
            None
        }
    };
    (kv_bettors, kv_golfers)
}

pub(super) fn select_setup_bettors(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    state: &mut ReplState,
    kv_bettors: Option<Vec<String>>,
) -> Result<()> {
    if let Some(kv_bettors) = kv_bettors {
        if kv_bettors.is_empty() {
            println!("No bettors found.");
        } else {
            for bettor in &kv_bettors {
                println!("{bettor}");
            }
        }
        prompt_for_bettors(rl, helper_state, state, kv_bettors)?;
    } else if !bettors_selection_exists(state) {
        handle_pick_bettors(rl, helper_state, state)?;
    }
    Ok(())
}

pub(super) fn resolve_setup_selections(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    state: &mut ReplState,
) -> Result<Vec<crate::repl::state::GolferSelection>> {
    if let Some(existing) = take_golfers_by_bettor(state) {
        if existing.is_empty() {
            return select_golfers_by_bettor(rl, helper_state, state, false, None, None);
        }
        return Ok(existing);
    }
    select_golfers_by_bettor(rl, helper_state, state, false, None, None)
}

pub(super) fn event_name_for_id(
    events: &[(String, String)],
    event_id: i64,
    raw_id: &str,
) -> String {
    let event_id_str = event_id.to_string();
    events
        .iter()
        .find(|(id, _)| id == &event_id_str)
        .map_or_else(|| raw_id.to_string(), |(_, name)| name.clone())
}

pub(super) fn load_golfers_for_payload(
    state: &mut ReplState,
    kv_golfers: Option<&Vec<(String, i64)>>,
) -> Result<Vec<(String, i64)>> {
    Ok(if let Some(kv_golfers) = kv_golfers {
        kv_golfers.clone()
    } else {
        crate::repl::state::load_cached_golfers(state)?
    })
}
