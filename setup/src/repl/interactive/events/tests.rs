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
