use crate::repl::commands::{
    CommandId, SubcommandId, build_repl_help, find_command, find_subcommand, print_subcommand_help,
};
use crate::repl::helper::{ReplCompletionMode, ReplHelper, ReplHelperState};
use crate::repl::parse::format_parse_error;
use crate::repl::prompt::{ReplPromptError, prompt_for_items};
use crate::repl::state::{
    GolferSelection, ReplState, bettors_selection_exists, ensure_list_bettors,
    ensure_list_events, eup_event_exists, has_cached_events, load_bettors_selection,
    load_cached_golfers, load_eup_json, load_event_golfers, output_json_path,
    persist_bettors_selection, print_list_event_error, set_golfers_by_bettor,
    take_golfers_by_bettor,
};
use anyhow::{Context, Result};
use chrono::Datelike;
use serde_json::Value;
use serde_json::json;
use rustyline::Editor;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

mod commands;
mod complete;
mod helper;
mod parse;
mod prompt;
mod state;

/// Run the interactive REPL for selecting new events.
///
/// # Errors
/// Returns an error if the REPL or ESPN interactions fail.
pub fn run_new_event_repl(
    eup_json: Option<PathBuf>,
    output_json: Option<PathBuf>,
) -> Result<()> {
    let help_text = build_repl_help();
    println!("Entering new_event mode. Press Ctrl-C or Ctrl-D to quit.");
    let mut rl = Editor::<ReplHelper, DefaultHistory>::new().context("init repl")?;
    let helper_state = Rc::new(RefCell::new(ReplHelperState::new()));
    rl.set_helper(Some(ReplHelper::new(Rc::clone(&helper_state))));
    let mut state = ReplState::new(eup_json, output_json).context("init repl state")?;
    loop {
        match rl.readline("new_event> ") {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                rl.add_history_entry(input)?;
                let mut parts = input.split_whitespace();
                let command_token = parts.next().unwrap_or_default();
                let Some(command) = find_command(command_token) else {
                    println!("Unknown command: {input}");
                    println!("{help_text}");
                    continue;
                };

                match command.id {
                    CommandId::Help => {
                        println!("{help_text}");
                    }
                    CommandId::ListEvents => {
                        let subcommand_token = parts.next();
                        let subcommand = subcommand_token
                            .and_then(|token| find_subcommand(command.subcommands, token));
                        if let Some(token) = subcommand_token
                            && subcommand.is_none()
                        {
                            println!("Unknown subcommand: {token}");
                            print_subcommand_help(command);
                            continue;
                        }
                        if matches!(subcommand.map(|sub| sub.id), Some(SubcommandId::Help)) {
                            print_subcommand_help(command);
                            continue;
                        }
                        let refresh =
                            matches!(subcommand.map(|sub| sub.id), Some(SubcommandId::Refresh));
                        match ensure_list_events(&mut state, refresh, true) {
                            Ok(events) => {
                                if events.is_empty() {
                                    println!("No events found.");
                                } else {
                                    for (id, name) in events {
                                        println!("{id} {name}");
                                    }
                                }
                            }
                            Err(err) => print_list_event_error(&err),
                        }
                    }
                    CommandId::GetAvailableGolfers => {
                        match ensure_list_events(&mut state, false, false) {
                        Ok(events) => {
                            if events.is_empty() {
                                println!("No events found.");
                            } else {
                                for (id, name) in &events {
                                    println!("{id} {name}");
                                }
                            }
                            let event_ids: Vec<String> =
                                events.iter().map(|(id, _)| id.clone()).collect();
                            helper_state.borrow_mut().set_mode(ReplCompletionMode::PromptItems {
                                items: event_ids,
                                quote_items: false,
                            });
                            let response =
                                prompt_for_items(&mut rl, "Which events? (csv or space-separated) ");
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
                                Err(ReplPromptError::Failed(err)) => {
                                    return Err(err);
                                }
                            }
                        }
                        Err(err) => print_list_event_error(&err),
                        }
                    }
                    CommandId::PickBettors => {
                        handle_pick_bettors(&mut rl, &helper_state, &mut state)?;
                    }
                    CommandId::SetGolfersByBettor => {
                        let selections =
                            select_golfers_by_bettor(&mut rl, &helper_state, &mut state, true)?;
                        set_golfers_by_bettor(&mut state, selections);
                    }
                    CommandId::SetupEvent => {
                        run_setup_event(&mut rl, &helper_state, &mut state)?;
                    }
                    CommandId::Exit | CommandId::Quit => break,
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(err) => return Err(err).context("read repl input"),
        }
    }
    Ok(())
}

fn handle_pick_bettors(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    state: &mut ReplState,
) -> Result<()> {
    let bettors = ensure_list_bettors(state)?;
    if bettors.is_empty() {
        println!("No bettors found.");
    } else {
        for bettor in &bettors {
            println!("{bettor}");
        }
    }
    helper_state.borrow_mut().set_mode(ReplCompletionMode::PromptItems {
        items: bettors,
        quote_items: false,
    });
    let response = prompt_for_items(rl, "Which bettors? (csv or space separated) ");
    helper_state.borrow_mut().set_mode(ReplCompletionMode::Repl);
    match response {
        Ok(selected) => {
            if selected.is_empty() {
                println!("No bettors selected.");
            } else {
                persist_bettors_selection(state, &selected)?;
                println!("{}", selected.join(" "));
            }
        }
        Err(ReplPromptError::Interrupted) => {}
        Err(ReplPromptError::Invalid(err, line)) => {
            println!("{}", format_parse_error(&line, err.index));
        }
        Err(ReplPromptError::Failed(err)) => {
            return Err(err);
        }
    }
    Ok(())
}

fn select_golfers_by_bettor(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    state: &mut ReplState,
    emit_output: bool,
) -> Result<Vec<GolferSelection>> {
    if !has_cached_events(state)? {
        println!("no events in cache; run list_events first.");
        return Ok(Vec::new());
    }

    if !bettors_selection_exists(state) {
        handle_pick_bettors(rl, helper_state, state)?;
    }

    if !bettors_selection_exists(state) {
        println!("No bettors selected.");
        return Ok(Vec::new());
    }

    let bettors = load_bettors_selection(state)?;
    if bettors.is_empty() {
        println!("No bettors selected.");
        return Ok(Vec::new());
    }

    let golfers = load_cached_golfers(state)?;
    if golfers.is_empty() {
        println!("No golfers found in cache.");
        return Ok(Vec::new());
    }
    let golfer_names: Vec<String> = golfers.iter().map(|(name, _)| name.clone()).collect();
    let golfer_lookup: std::collections::BTreeMap<String, i64> = golfers.into_iter().collect();

    let mut selections = Vec::new();
    for bettor in bettors {
        helper_state.borrow_mut().set_mode(ReplCompletionMode::PromptItems {
            items: golfer_names.clone(),
            quote_items: true,
        });
        let prompt = format!(
            "Which golfers for {bettor}? (csv or space separated, quote-delimited) "
        );
        let response = prompt_for_items(rl, &prompt);
        helper_state.borrow_mut().set_mode(ReplCompletionMode::Repl);
        match response {
            Ok(selected) => {
                let mut entries = Vec::new();
                for golfer in selected {
                    match golfer_lookup.get(&golfer) {
                        Some(id) => {
                            let selection = GolferSelection {
                                bettor: bettor.clone(),
                                golfer_espn_id: *id,
                            };
                            entries.push(selection.clone());
                            selections.push(selection);
                        }
                        None => {
                            println!("Unknown golfer: {golfer}");
                        }
                    }
                }
                if emit_output {
                    let payload: Vec<Value> = entries
                        .iter()
                        .map(|entry| {
                            json!({
                                "bettor": entry.bettor,
                                "golfer_espn_id": entry.golfer_espn_id,
                            })
                        })
                        .collect();
                    println!("{}", serde_json::to_string(&payload)?);
                }
            }
            Err(ReplPromptError::Interrupted) => {}
            Err(ReplPromptError::Invalid(err, line)) => {
                println!("{}", format_parse_error(&line, err.index));
            }
            Err(ReplPromptError::Failed(err)) => {
                return Err(err);
            }
        }
    }

    Ok(selections)
}

fn run_setup_event(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    state: &mut ReplState,
) -> Result<()> {
    let events = ensure_list_events(state, false, true)?;
    if events.is_empty() {
        println!("No events found.");
        return Ok(());
    }
    for (id, name) in &events {
        println!("{id} {name}");
    }

    let event_ids: Vec<String> = events.iter().map(|(id, _)| id.clone()).collect();
    helper_state.borrow_mut().set_mode(ReplCompletionMode::PromptItems {
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
        Err(ReplPromptError::Failed(err)) => {
            return Err(err);
        }
    };
    let Some(event_id_raw) = selected_event else {
        return Ok(());
    };
    let event_id: i64 = match event_id_raw.parse() {
        Ok(id) => id,
        Err(_) => {
            println!("Invalid event id: {event_id_raw}");
            return Ok(());
        }
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
    let existing = load_eup_json(state)?;
    let event_name = events
        .iter()
        .find(|(id, _)| *id == event_id_raw)
        .map(|(_, name)| name.clone())
        .unwrap_or_else(|| event_id_raw.clone());
    let golfers = load_event_golfers(state, &event_id_raw)?;
    let bettors = load_bettors_selection(state)?;
    let year = chrono::Utc::now().year();
    let event_user_player: Vec<Value> = selections
        .iter()
        .map(|entry| {
            json!({
                "bettor": entry.bettor,
                "golfer_espn_id": entry.golfer_espn_id,
            })
        })
        .collect();
    let golfers_payload: Vec<Value> = golfers
        .iter()
        .map(|(name, id)| {
            json!({
                "name": name,
                "espn_id": id,
            })
        })
        .collect();
    let new_event = json!({
        "event": event_id,
        "year": year,
        "name": event_name,
        "score_view_step_factor": 3.0,
        "data_to_fill_if_event_and_year_missing": [
            {
                "bettors": bettors,
                "golfers": golfers_payload,
                "event_user_player": event_user_player,
            }
        ],
    });

    let mut payload = existing;
    payload.push(new_event);
    let serialized = serde_json::to_string_pretty(&payload)?;
    std::fs::write(&output_path, serialized)
        .with_context(|| format!("write {}", output_path.display()))?;
    println!("Wrote {}", output_path.display());
    Ok(())
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
                    .filter_map(|entry| entry.ok())
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
