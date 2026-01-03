use crate::repl::commands::{
    CommandId, build_repl_help, find_command,
};
use crate::repl::helper::{ReplHelper, ReplHelperState};
use crate::repl::state::ReplState;
use anyhow::{Context, Result};
use rustyline::Editor;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

mod bettors;
mod events;
mod golfers;
mod setup;

use bettors::handle_pick_bettors;
use events::{handle_get_available_golfers, handle_list_events_command};
use golfers::select_golfers_by_bettor;
use setup::run_setup_event;

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
                if handle_repl_line(&line, &mut rl, &helper_state, &mut state, &help_text)?
                    == ReplFlow::Exit
                {
                    break;
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(err) => return Err(err).context("read repl input"),
        }
    }
    Ok(())
}

#[derive(PartialEq)]
enum ReplFlow {
    Continue,
    Exit,
}

fn handle_repl_line(
    line: &str,
    rl: &mut Editor<ReplHelper, DefaultHistory>,
    helper_state: &Rc<RefCell<ReplHelperState>>,
    state: &mut ReplState,
    help_text: &str,
) -> Result<ReplFlow> {
    let input = line.trim();
    if input.is_empty() {
        return Ok(ReplFlow::Continue);
    }
    rl.add_history_entry(input)?;
    let mut parts = input.split_whitespace();
    let command_token = parts.next().unwrap_or_default();
    let Some(command) = find_command(command_token) else {
        println!("Unknown command: {input}");
        println!("{help_text}");
        return Ok(ReplFlow::Continue);
    };

    match command.id {
        CommandId::Help => {
            println!("{help_text}");
        }
        CommandId::ListEvents => {
            handle_list_events_command(state, command, parts.next())?;
        }
        CommandId::GetAvailableGolfers => {
            handle_get_available_golfers(rl, helper_state, state)?;
        }
        CommandId::PickBettors => {
            handle_pick_bettors(rl, helper_state, state)?;
        }
        CommandId::SetGolfersByBettor => {
            let selections =
                select_golfers_by_bettor(rl, helper_state, state, true)?;
            crate::repl::state::set_golfers_by_bettor(state, selections);
        }
        CommandId::SetupEvent => {
            run_setup_event(rl, helper_state, state)?;
        }
        CommandId::Exit | CommandId::Quit => return Ok(ReplFlow::Exit),
    }
    Ok(ReplFlow::Continue)
}
