use super::bettors::handle_pick_bettors;
use super::golfers::select_golfers_by_bettor;
use crate::repl::helper::{ReplCompletionMode, ReplHelper, ReplHelperState};
use crate::repl::parse::format_parse_error;
use crate::repl::payload::write_event_payload;
use crate::repl::prompt::{ReplPromptError, prompt_for_items};
use crate::repl::state::{
    ReplState, bettors_selection_exists, ensure_list_events, eup_event_exists,
    load_bettors_selection, load_event_golfers, output_json_path, take_golfers_by_bettor,
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

    if !bettors_selection_exists(state) {
        handle_pick_bettors(rl, helper_state, state)?;
    }

    let selections = if let Some(existing) = take_golfers_by_bettor(state) {
        existing
    } else {
        let selections = select_golfers_by_bettor(rl, helper_state, state, false)?;
        if selections.is_empty() {
            return Ok(());
        }
        selections
    };

    let output_path = ensure_output_path(rl, helper_state, state)?;
    let event_name = events
        .iter()
        .find(|(id, _)| *id == event_id_raw)
        .map_or_else(|| event_id_raw.clone(), |(_, name)| name.clone());
    let golfers = load_event_golfers(state, &event_id_raw)?;
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

fn print_events(events: &[(String, String)]) {
    if events.is_empty() {
        println!("No events found.");
        return;
    }
    for (id, name) in events {
        println!("{id} {name}");
    }
}

fn ensure_output_path(
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
