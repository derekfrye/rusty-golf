use crate::repl::commands::{
    CommandId, SubcommandId, build_repl_help, find_command, find_subcommand, print_subcommand_help,
};
use crate::repl::helper::{ReplCompletionMode, ReplHelper, ReplHelperState};
use crate::repl::prompt::{ReplPromptError, prompt_for_items};
use crate::repl::parse::format_parse_error;
use crate::repl::state::{
    ReplState, ensure_list_bettors, ensure_list_events, persist_bettors_selection,
    print_list_event_error,
};
use anyhow::{Context, Result};
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
pub fn run_new_event_repl(eup_json: Option<PathBuf>) -> Result<()> {
    let help_text = build_repl_help();
    println!("Entering new_event mode. Press Ctrl-C or Ctrl-D to quit.");
    let mut rl = Editor::<ReplHelper, DefaultHistory>::new().context("init repl")?;
    let helper_state = Rc::new(RefCell::new(ReplHelperState::new()));
    rl.set_helper(Some(ReplHelper::new(Rc::clone(&helper_state))));
    let mut state = ReplState::new(eup_json).context("init repl state")?;
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
                        match ensure_list_events(&mut state, refresh) {
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
                    CommandId::GetAvailableGolfers => match ensure_list_events(&mut state, false) {
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
                            helper_state
                                .borrow_mut()
                                .set_mode(ReplCompletionMode::PromptItems(event_ids));
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
                    },
                    CommandId::PickBettors => {
                        let bettors = ensure_list_bettors(&mut state)?;
                        if bettors.is_empty() {
                            println!("No bettors found.");
                        } else {
                            for bettor in &bettors {
                                println!("{bettor}");
                            }
                        }
                        helper_state
                            .borrow_mut()
                            .set_mode(ReplCompletionMode::PromptItems(bettors));
                        let response =
                            prompt_for_items(&mut rl, "Which bettors? (csv or space separated) ");
                        helper_state.borrow_mut().set_mode(ReplCompletionMode::Repl);
                        match response {
                            Ok(selected) => {
                                if selected.is_empty() {
                                    println!("No bettors selected.");
                                } else {
                                    persist_bettors_selection(&state, &selected)?;
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
                    CommandId::Exit | CommandId::Quit => break,
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(err) => return Err(err).context("read repl input"),
        }
    }
    Ok(())
}
