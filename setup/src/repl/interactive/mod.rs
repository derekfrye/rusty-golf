use crate::repl::commands::{CommandId, build_repl_help, find_command};
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
mod update;

use bettors::handle_pick_bettors;
use events::{handle_get_available_golfers, handle_get_event_details, handle_list_events_command};
use golfers::select_golfers_by_bettor;
use setup::run_setup_event;
use update::run_update_event;

/// Run the interactive REPL for selecting new events.
///
/// # Errors
/// Returns an error if the REPL or ESPN interactions fail.
pub fn run_new_event_repl(
    eup_json: Option<PathBuf>,
    output_json: Option<PathBuf>,
    kv_access: Option<crate::config::KvAccessConfig>,
) -> Result<()> {
    run_repl("new_event", eup_json, output_json, kv_access)
}

/// Run the interactive REPL for updating events.
///
/// # Errors
/// Returns an error if the REPL or ESPN/KV interactions fail.
pub fn run_update_event_repl(
    eup_json: Option<PathBuf>,
    output_json: Option<PathBuf>,
    kv_access: crate::config::KvAccessConfig,
) -> Result<()> {
    run_repl("update_event", eup_json, output_json, Some(kv_access))
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
        println!("{}", build_repl_help(state.expert_enabled));
        return Ok(ReplFlow::Continue);
    };

    if command.expert_only && !state.expert_enabled {
        println!("Toggle on expert mode to use command `{}`.", command_token);
        return Ok(ReplFlow::Continue);
    }

    match command.id {
        CommandId::Help => {
            println!("{}", build_repl_help(state.expert_enabled));
        }
        CommandId::ListEvents => {
            handle_list_events_command(state, command, parts.next());
        }
        CommandId::GetEventDetails => {
            handle_get_event_details(rl, helper_state, state)?;
        }
        CommandId::GetAvailableGolfers => {
            handle_get_available_golfers(rl, helper_state, state)?;
        }
        CommandId::PickBettors => {
            handle_pick_bettors(rl, helper_state, state)?;
        }
        CommandId::SetGolfersByBettor => {
            let selections =
                select_golfers_by_bettor(rl, helper_state, state, true, None, None)?;
            if selections.is_empty() {
                println!("No golfers selected.");
            } else {
                crate::repl::state::set_golfers_by_bettor(state, selections);
            }
        }
        CommandId::SetupEvent => {
            run_setup_event(rl, helper_state, state)?;
        }
        CommandId::UpdateEvent => {
            run_update_event(rl, helper_state, state)?;
        }
        CommandId::Expert => {
            state.expert_enabled = !state.expert_enabled;
            helper_state
                .borrow_mut()
                .set_expert_enabled(state.expert_enabled);
            if state.expert_enabled {
                println!("Expert mode enabled.");
            } else {
                println!("Expert mode disabled.");
            }
        }
        CommandId::Exit | CommandId::Quit => return Ok(ReplFlow::Exit),
    }
    Ok(ReplFlow::Continue)
}

fn run_repl(
    prompt: &str,
    eup_json: Option<PathBuf>,
    output_json: Option<PathBuf>,
    kv_access: Option<crate::config::KvAccessConfig>,
) -> Result<()> {
    println!("Entering {prompt} mode. Press Ctrl-C or Ctrl-D to quit.");
    let mut rl = Editor::<ReplHelper, DefaultHistory>::new().context("init repl")?;
    let helper_state = Rc::new(RefCell::new(ReplHelperState::new()));
    rl.set_helper(Some(ReplHelper::new(Rc::clone(&helper_state))));
    let mut state =
        ReplState::new(eup_json, output_json, kv_access).context("init repl state")?;
    helper_state
        .borrow_mut()
        .set_expert_enabled(state.expert_enabled);
    let prompt = format!("{prompt}> ");
    loop {
        match rl.readline(&prompt) {
            Ok(line) => {
                if handle_repl_line(&line, &mut rl, &helper_state, &mut state)?
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
