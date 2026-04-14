use crate::repl::helper::{ReplCompletionMode, ReplHelper, ReplHelperState};
use crate::repl::payload::write_event_payload;
use crate::repl::state::{
    EventListMode, ReplState, ensure_list_events, eup_event_exists, load_bettors_selection,
    output_json_path,
};
use anyhow::{Context, Result};
use rustyline::Editor;
use rustyline::history::DefaultHistory;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use super::setup_helpers::{
    event_name_for_id, load_golfers_for_payload, load_kv_seed_data, resolve_setup_selections,
    select_setup_bettors, select_setup_event,
};

pub(super) fn run_setup_event(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    state: &mut ReplState,
) -> Result<()> {
    let events = ensure_list_events(state, EventListMode::EnsureAll, true)?;
    print_events(&events);
    if events.is_empty() {
        return Ok(());
    }

    let Some((event_id, event_id_raw)) = select_setup_event(rl, helper_state, &events)? else {
        return Ok(());
    };
    if eup_event_exists(state, event_id)? {
        println!("Warning: event {event_id} already exists in eup json.");
    }

    let (kv_bettors, kv_golfers) = load_kv_seed_data(state, event_id);
    select_setup_bettors(rl, helper_state, state, kv_bettors)?;
    let selections = resolve_setup_selections(rl, helper_state, state)?;
    if selections.is_empty() {
        return Ok(());
    }

    let output_path = ensure_output_path(rl, helper_state, state)?;
    let event_name = event_name_for_id(&events, event_id, &event_id_raw);
    let golfers = load_golfers_for_payload(state, kv_golfers.as_ref())?;
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
