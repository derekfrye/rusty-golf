use super::bettors::{handle_pick_bettors, prompt_for_bettors};
use super::golfers::select_golfers_by_bettor;
use crate::repl::helper::{ReplCompletionMode, ReplHelper, ReplHelperState};
use crate::repl::parse::format_parse_error;
use crate::repl::payload::write_event_payload;
use crate::repl::prompt::{ReplPromptError, prompt_for_items};
use crate::repl::state::{
    ReplState, bettors_selection_exists, ensure_list_events, eup_event_exists,
    load_bettors_selection, load_kv_bettors, load_kv_golfers_list,
    output_json_path, take_golfers_by_bettor,
};
use anyhow::{Context, Result};
use rustyline::Editor;
use rustyline::history::DefaultHistory;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub(super) fn run_setup_event(
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
        return Ok(());
    };
    let event_id: i64 = if let Ok(id) = event_id_raw.parse() {
        id
    } else {
        println!("Invalid event id: {event_id_raw}");
        return Ok(());
    };
    if eup_event_exists(state, event_id)? {
        println!("Warning: event {event_id} already exists in eup json.");
    }

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
    if let Some(kv_bettors) = kv_bettors.clone() {
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

    let selections = if let Some(existing) = take_golfers_by_bettor(state) {
        if existing.is_empty() {
            select_golfers_by_bettor(rl, helper_state, state, false, None, None)?
        } else {
            existing
        }
    } else {
        select_golfers_by_bettor(rl, helper_state, state, false, None, None)?
    };
    if selections.is_empty() {
        return Ok(());
    }

    let output_path = ensure_output_path(rl, helper_state, state)?;
    let event_name = events
        .iter()
        .find(|(id, _)| *id == event_id_raw)
        .map_or_else(|| event_id_raw.clone(), |(_, name)| name.clone());
    let golfers = if let Some(kv_golfers) = kv_golfers.as_ref() {
        kv_golfers.clone()
    } else {
        crate::repl::state::load_cached_golfers(state)?
    };
    let bettors = load_bettors_selection(state)?;
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

pub(super) fn print_events(events: &[(String, String)]) {
    if events.is_empty() {
        println!("No events found.");
        return;
    }
    for (id, name) in events {
        println!("{id} {name}");
    }
}

pub(super) fn ensure_output_path(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    state: &ReplState,
) -> Result<PathBuf> {
    if let Some(path) = output_json_path(state) {
        if path.exists() {
            let confirm = rl
                .readline("File exists. Overwrite? (y/N) ")
                .context("read overwrite confirmation")?;
            if confirm.trim().eq_ignore_ascii_case("y") {
                return Ok(path);
            }
        } else {
            return Ok(path);
        }
    }

    loop {
        let entries = std::env::current_dir()
            .ok()
            .and_then(|dir| std::fs::read_dir(dir).ok())
            .map(|read_dir| {
                read_dir
                    .filter_map(Result::ok)
                    .filter_map(|entry| {
                        let file_type = entry.file_type().ok()?;
                        if file_type.is_dir() {
                            None
                        } else {
                            Some(entry.path())
                        }
                    })
                    .collect::<Vec<PathBuf>>()
            })
            .unwrap_or_default();
        helper_state
            .borrow_mut()
            .set_mode(ReplCompletionMode::PromptPaths { items: entries });
        let read = rl.readline("Output filename? ");
        helper_state.borrow_mut().set_mode(ReplCompletionMode::Repl);
        let path = read.context("read output filename")?;
        let trimmed = path.trim();
        if trimmed.is_empty() {
            continue;
        }
        let candidate = PathBuf::from(trimmed);
        if candidate.exists() {
            let confirm = rl
                .readline("File exists. Overwrite? (y/N) ")
                .context("read overwrite confirmation")?;
            if confirm.trim().eq_ignore_ascii_case("y") {
                return Ok(candidate);
            }
            continue;
        }
        return Ok(candidate);
    }
}
