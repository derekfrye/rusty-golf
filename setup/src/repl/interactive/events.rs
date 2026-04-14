use crate::event_details::build_event_details_row;
use crate::repl::commands::{ReplCommand, SubcommandId, find_subcommand, print_subcommand_help};
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

pub(super) fn handle_list_events_command(
    state: &mut ReplState,
    command: &ReplCommand,
    tokens: &[&str],
) {
    let Some((mode, warm_cache)) = parse_list_events_mode(command, tokens) else {
        return;
    };
    if matches!(mode, ListEventsAction::Help) {
        print_subcommand_help(command);
        return;
    }
    let mode = match mode {
        ListEventsAction::Help => unreachable!(),
        ListEventsAction::EnsureAll => EventListMode::EnsureAll,
        ListEventsAction::RefreshEspn => EventListMode::RefreshEspn,
        ListEventsAction::RefreshKv => EventListMode::RefreshKv,
        ListEventsAction::RefreshAll => EventListMode::RefreshAll,
    };
    match ensure_list_events(state, mode, warm_cache) {
        Ok(events) => print_events(&events),
        Err(err) => print_list_event_error(&err),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ListEventsAction {
    Help,
    EnsureAll,
    RefreshEspn,
    RefreshKv,
    RefreshAll,
}

fn parse_list_events_mode(
    command: &ReplCommand,
    tokens: &[&str],
) -> Option<(ListEventsAction, bool)> {
    if tokens.is_empty() {
        return Some((ListEventsAction::EnsureAll, true));
    }

    let token = tokens[0];
    let Some(subcommand) = find_subcommand(command.subcommands, token) else {
        println!("Unknown subcommand: {token}");
        print_subcommand_help(command);
        return None;
    };

    match subcommand.id {
        SubcommandId::Help => {
            if tokens.len() > 1 {
                println!("Unexpected argument: {}", tokens[1]);
                print_subcommand_help(command);
                return None;
            }
            Some((ListEventsAction::Help, false))
        }
        SubcommandId::Kv => {
            if tokens.len() > 1 {
                println!("Unexpected argument: {}", tokens[1]);
                print_subcommand_help(command);
                return None;
            }
            Some((ListEventsAction::RefreshKv, false))
        }
        SubcommandId::Refresh => match tokens.get(1).copied() {
            None => Some((ListEventsAction::RefreshEspn, true)),
            Some("espn") => Some((ListEventsAction::RefreshEspn, true)),
            Some("all") => Some((ListEventsAction::RefreshAll, true)),
            Some("kv") => Some((ListEventsAction::RefreshKv, false)),
            Some(other) => {
                println!("Unknown refresh target: {other}");
                print_subcommand_help(command);
                None
            }
        },
    }
}

pub(super) fn handle_get_available_golfers(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    state: &mut ReplState,
) -> Result<()> {
    match ensure_list_events(state, EventListMode::EnsureAll, false) {
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

#[cfg(test)]
mod tests {
    use super::{ListEventsAction, parse_list_events_mode};
    use crate::repl::commands::REPL_COMMANDS;

    fn list_events_command() -> &'static crate::repl::commands::ReplCommand {
        REPL_COMMANDS
            .iter()
            .find(|command| command.name == "list_events")
            .unwrap()
    }

    #[test]
    fn refresh_defaults_to_espn_only() {
        let parsed = parse_list_events_mode(list_events_command(), &["refresh"]);
        assert_eq!(parsed, Some((ListEventsAction::RefreshEspn, true)));
    }

    #[test]
    fn refresh_all_is_distinct() {
        let parsed = parse_list_events_mode(list_events_command(), &["refresh", "all"]);
        assert_eq!(parsed, Some((ListEventsAction::RefreshAll, true)));
    }

    #[test]
    fn refresh_kv_alias_is_distinct() {
        let parsed = parse_list_events_mode(list_events_command(), &["refresh", "kv"]);
        assert_eq!(parsed, Some((ListEventsAction::RefreshKv, false)));
    }

    #[test]
    fn kv_refresh_is_distinct() {
        let parsed = parse_list_events_mode(list_events_command(), &["kv"]);
        assert_eq!(parsed, Some((ListEventsAction::RefreshKv, false)));
    }
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
