use crate::repl::commands::{REPL_COMMANDS, find_command};
use crate::repl::complete::{complete_items_prompt, complete_path_prompt};
use rustyline::Helper;
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub(crate) enum ReplCompletionMode {
    Repl,
    PromptItems {
        items: Vec<String>,
        quote_items: bool,
    },
    PromptPaths {
        items: Vec<std::path::PathBuf>,
    },
}

pub(crate) struct ReplHelperState {
    mode: ReplCompletionMode,
    expert_enabled: bool,
}

impl ReplHelperState {
    pub(crate) fn new() -> Self {
        Self {
            mode: ReplCompletionMode::Repl,
            expert_enabled: false,
        }
    }

    pub(crate) fn set_mode(&mut self, mode: ReplCompletionMode) {
        self.mode = mode;
    }

    pub(crate) fn set_expert_enabled(&mut self, enabled: bool) {
        self.expert_enabled = enabled;
    }

    pub(crate) fn expert_enabled(&self) -> bool {
        self.expert_enabled
    }
}

pub(crate) struct ReplHelper {
    state: Rc<RefCell<ReplHelperState>>,
}

impl ReplHelper {
    pub(crate) fn new(state: Rc<RefCell<ReplHelperState>>) -> Self {
        Self { state }
    }
}

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let mode = self.state.borrow().mode.clone();
        match mode {
            ReplCompletionMode::Repl => Ok(Self::complete_repl(
                line,
                pos,
                self.state.borrow().expert_enabled(),
            )),
            ReplCompletionMode::PromptItems { items, quote_items } => {
                Ok(complete_items_prompt(line, pos, &items, quote_items))
            }
            ReplCompletionMode::PromptPaths { items } => {
                Ok(complete_path_prompt(line, pos, &items))
            }
        }
    }
}

impl ReplHelper {
    fn complete_repl(line: &str, pos: usize, expert_enabled: bool) -> (usize, Vec<Pair>) {
        let prefix = &line[..pos];
        let mut parts = prefix.split_whitespace();
        let first = parts.next().unwrap_or_default();
        let second = parts.next();
        let third = parts.next();

        if let Some(command) = find_command(first)
            && command.name == "list_events"
            && prefix.contains(char::is_whitespace)
        {
            let start = prefix.rfind(' ').map_or(pos, |i| i + 1);

            if second == Some("refresh")
                && (third.is_some() || prefix.ends_with(char::is_whitespace))
            {
                let target_prefix = third.unwrap_or_default();
                let candidates = ["kv", "all", "espn"]
                    .into_iter()
                    .filter(|candidate| candidate.starts_with(target_prefix))
                    .map(|candidate| Pair {
                        display: candidate.to_string(),
                        replacement: candidate.to_string(),
                    })
                    .collect();
                return (start, candidates);
            }

            let sub_prefix = if prefix.ends_with(char::is_whitespace) {
                ""
            } else {
                second.unwrap_or_default()
            };
            let candidates = ["help", "refresh"]
                .into_iter()
                .filter(|candidate| candidate.starts_with(sub_prefix))
                .map(|candidate| Pair {
                    display: candidate.to_string(),
                    replacement: candidate.to_string(),
                })
                .collect();
            return (start, candidates);
        }

        if let Some(command) = find_command(first)
            && !command.subcommands.is_empty()
            && second.is_none()
            && prefix.contains(char::is_whitespace)
        {
            let sub_prefix = prefix.trim_start_matches(command.name).trim_start();
            let candidates = command
                .subcommands
                .iter()
                .map(|subcommand| subcommand.name)
                .filter(|cmd| cmd.starts_with(sub_prefix))
                .map(|cmd| Pair {
                    display: cmd.to_string(),
                    replacement: cmd.to_string(),
                })
                .collect();
            let start = prefix.rfind(' ').map_or(pos, |i| i + 1);
            return (start, candidates);
        }

        if prefix.contains(char::is_whitespace) {
            return (pos, Vec::new());
        }

        let candidates = REPL_COMMANDS
            .iter()
            .filter(|command| !command.expert_only || expert_enabled)
            .flat_map(|command| command.aliases.iter().copied().chain([command.name]))
            .filter(|cmd| cmd.starts_with(prefix))
            .map(|cmd| Pair {
                display: cmd.to_string(),
                replacement: cmd.to_string(),
            })
            .collect();
        (0, candidates)
    }
}

impl Hinter for ReplHelper {
    type Hint = String;
}

impl Highlighter for ReplHelper {}

impl Validator for ReplHelper {}

impl Helper for ReplHelper {}
