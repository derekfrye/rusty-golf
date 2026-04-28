use crate::event_details::build_event_details_row;
use crate::repl::helper::{ReplCompletionMode, ReplHelper, ReplHelperState};
use crate::repl::parse::format_parse_error;
use crate::repl::prompt::{ReplPromptError, prompt_for_items};
use crate::repl::state::{
    EventListMode, ReplState, ensure_list_events, load_eup_event_dates, print_list_event_error,
};
use anyhow::Result;
use rustyline::Editor;
use rustyline::history::DefaultHistory;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use tabled::Table;

use super::print_events;

pub(in crate::repl::interactive) fn handle_get_event_details(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    state: &mut ReplState,
) -> Result<()> {
    let events = match ensure_list_events(state, EventListMode::EnsureAll, false) {
        Ok(events) => {
            print_events(&events);
            events
        }
        Err(err) => {
            print_list_event_error(&err);
            return Ok(());
        }
    };

    let selected = prompt_for_event_ids(rl, helper_state, &events)?;
    if selected.is_empty() {
        println!("No events selected.");
        return Ok(());
    }

    let event_lookup: BTreeMap<&str, &str> = events
        .iter()
        .map(|(id, name)| (id.as_str(), name.as_str()))
        .collect();
    let eup_dates = match load_eup_event_dates(state) {
        Ok(dates) => Some(dates),
        Err(err) => {
            println!("Warning: failed to load eup dates: {err}");
            None
        }
    };
    let mut rows = Vec::new();
    for raw_id in selected {
        let event_id: i64 = if let Ok(value) = raw_id.parse() {
            value
        } else {
            println!("Invalid event id: {raw_id}");
            continue;
        };
        let event_name_hint = event_lookup.get(raw_id.as_str()).copied();
        let eup_dates = eup_dates.as_ref().and_then(|dates| dates.get(&event_id));
        match build_event_details_row(
            event_id,
            event_name_hint,
            state.espn.as_ref(),
            &state.event_cache_dir,
            eup_dates,
        ) {
            Ok(row) => rows.push(row),
            Err(err) => println!("Failed to load event {event_id}: {err}"),
        }
    }

    if rows.is_empty() {
        println!("No event details found.");
    } else {
        println!("{}", Table::new(rows));
    }
    Ok(())
}

fn prompt_for_event_ids(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    events: &[(String, String)],
) -> Result<Vec<String>> {
    let event_ids: Vec<String> = events.iter().map(|(id, _)| id.clone()).collect();
    helper_state
        .borrow_mut()
        .set_mode(ReplCompletionMode::PromptItems {
            items: event_ids,
            quote_items: false,
        });
    let response = prompt_for_items(rl, "Which events? (csv or space-separated) ");
    helper_state.borrow_mut().set_mode(ReplCompletionMode::Repl);
    match response {
        Ok(selected) => Ok(selected),
        Err(ReplPromptError::Interrupted) => Ok(Vec::new()),
        Err(ReplPromptError::Invalid(err, line)) => {
            println!("{}", format_parse_error(&line, err.index));
            Ok(Vec::new())
        }
        Err(ReplPromptError::Failed(err)) => Err(err),
    }
}
