use crate::repl::helper::{ReplCompletionMode, ReplHelper, ReplHelperState};
use crate::repl::parse::format_parse_error;
use crate::repl::prompt::{ReplPromptError, prompt_for_items};
use crate::repl::state::{ReplState, ensure_list_bettors, persist_bettors_selection};
use anyhow::Result;
use rustyline::Editor;
use rustyline::history::DefaultHistory;
use std::cell::RefCell;
use std::rc::Rc;

pub(super) fn handle_pick_bettors(
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
    helper_state
        .borrow_mut()
        .set_mode(ReplCompletionMode::PromptItems {
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
        Err(ReplPromptError::Failed(err)) => return Err(err),
    }
    Ok(())
}
