use crate::repl::commands::{ReplCommand, SubcommandId, find_subcommand, print_subcommand_help};
use crate::event_details::build_event_details_row;
use crate::repl::helper::{ReplCompletionMode, ReplHelper, ReplHelperState};
use crate::repl::parse::format_parse_error;
use crate::repl::prompt::{ReplPromptError, prompt_for_items};
use crate::repl::state::{ReplState, ensure_list_events, print_list_event_error};
use anyhow::Result;
use rustyline::Editor;
use rustyline::history::DefaultHistory;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use tabled::Table;

pub(super) fn handle_list_events_command(
    state: &mut ReplState,
    command: &ReplCommand,
    token: Option<&str>,
) {
    let subcommand = token.and_then(|token| find_subcommand(command.subcommands, token));
    if let Some(token) = token
        && subcommand.is_none()
    {
        println!("Unknown subcommand: {token}");
        print_subcommand_help(command);
        return;
    }
    if matches!(subcommand.map(|sub| sub.id), Some(SubcommandId::Help)) {
        print_subcommand_help(command);
        return;
    }
    let refresh = matches!(subcommand.map(|sub| sub.id), Some(SubcommandId::Refresh));
    match ensure_list_events(state, refresh, true) {
        Ok(events) => print_events(&events),
        Err(err) => print_list_event_error(&err),
    }
}

pub(super) fn handle_get_available_golfers(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    state: &mut ReplState,
) -> Result<()> {
    match ensure_list_events(state, false, false) {
        Ok(events) => {
            print_events(&events);
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
                Ok(selected) => {
                    if selected.is_empty() {
                        println!("No events selected.");
                    } else {
                        println!("{}", selected.join(" "));
                    }
                }
                Err(ReplPromptError::Interrupted) => {}
                Err(ReplPromptError::Invalid(err, line)) => {
                    println!("{}", format_parse_error(&line, err.index));
                }
                Err(ReplPromptError::Failed(err)) => return Err(err),
            }
        }
        Err(err) => print_list_event_error(&err),
    }
    Ok(())
}

pub(super) fn handle_get_event_details(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    state: &mut ReplState,
) -> Result<()> {
    let events = match ensure_list_events(state, false, false) {
        Ok(events) => {
            print_events(&events);
            events
        }
        Err(err) => {
            print_list_event_error(&err);
            return Ok(());
        }
    };

    let event_ids: Vec<String> = events.iter().map(|(id, _)| id.clone()).collect();
    helper_state
        .borrow_mut()
        .set_mode(ReplCompletionMode::PromptItems {
            items: event_ids,
            quote_items: false,
        });
    let response = prompt_for_items(rl, "Which events? (csv or space-separated) ");
    helper_state.borrow_mut().set_mode(ReplCompletionMode::Repl);
    let selected = match response {
        Ok(selected) => selected,
        Err(ReplPromptError::Interrupted) => return Ok(()),
        Err(ReplPromptError::Invalid(err, line)) => {
            println!("{}", format_parse_error(&line, err.index));
            return Ok(());
        }
        Err(ReplPromptError::Failed(err)) => return Err(err),
    };
    if selected.is_empty() {
        println!("No events selected.");
        return Ok(());
    }

    let event_lookup: BTreeMap<&str, &str> = events
        .iter()
        .map(|(id, name)| (id.as_str(), name.as_str()))
        .collect();
    let mut rows = Vec::new();
    for raw_id in selected {
        let event_id: i64 = if let Ok(value) = raw_id.parse() {
            value
        } else {
            println!("Invalid event id: {raw_id}");
            continue;
        };
        let event_name_hint = event_lookup.get(raw_id.as_str()).copied();
        match build_event_details_row(
            event_id,
            event_name_hint,
            state.espn.as_ref(),
            &state.event_cache_dir,
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

fn print_events(events: &[(String, String)]) {
    if events.is_empty() {
        println!("No events found.");
        return;
    }
    for (id, name) in events {
        println!("{id} {name}");
    }
}
