use crate::espn::{fetch_event_names_parallel, list_espn_events, MalformedEspnJson};
use crate::repl::complete::complete_event_prompt;
use anyhow::{Context, Result};
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::Editor;
use rustyline::Helper;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use tempfile::TempDir;

mod complete;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommandId {
    Help,
    ListEvents,
    GetAvailableGolfers,
    Exit,
    Quit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SubcommandId {
    Help,
    Refresh,
}

struct ReplCommand {
    id: CommandId,
    name: &'static str,
    description: &'static str,
    aliases: &'static [&'static str],
    subcommands: &'static [ReplSubcommand],
}

struct ReplSubcommand {
    id: SubcommandId,
    name: &'static str,
    description: &'static str,
}

const LIST_EVENTS_SUBCOMMANDS: &[ReplSubcommand] = &[
    ReplSubcommand {
        id: SubcommandId::Help,
        name: "help",
        description: "display this help screen",
    },
    ReplSubcommand {
        id: SubcommandId::Refresh,
        name: "refresh",
        description: "if passed, hit espn api again to refresh current events.",
    },
];

const REPL_COMMANDS: &[ReplCommand] = &[
    ReplCommand {
        id: CommandId::Help,
        name: "help",
        description: "Show this help.",
        aliases: &["?", "-h", "--help"],
        subcommands: &[],
    },
    ReplCommand {
        id: CommandId::ListEvents,
        name: "list_events",
        description: "List events on ESPN API.",
        aliases: &[],
        subcommands: LIST_EVENTS_SUBCOMMANDS,
    },
    ReplCommand {
        id: CommandId::GetAvailableGolfers,
        name: "get_available_golfers",
        description: "Prompt for event IDs to use for golfers.",
        aliases: &[],
        subcommands: &[],
    },
    ReplCommand {
        id: CommandId::Exit,
        name: "exit",
        description: "Exit the REPL.",
        aliases: &[],
        subcommands: &[],
    },
    ReplCommand {
        id: CommandId::Quit,
        name: "quit",
        description: "Exit the REPL.",
        aliases: &[],
        subcommands: &[],
    },
];

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
                let command = match find_command(command_token) {
                    Some(command) => command,
                    None => {
                        println!("Unknown command: {input}");
                        println!("{help_text}");
                        continue;
                    }
                };

                match command.id {
                    CommandId::Help => {
                        println!("{help_text}");
                    }
                    CommandId::ListEvents => {
                        let subcommand_token = parts.next();
                        let subcommand = subcommand_token
                            .and_then(|token| find_subcommand(command.subcommands, token));
                        if let Some(token) = subcommand_token && subcommand.is_none() {
                            println!("Unknown subcommand: {}", token);
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
                    CommandId::GetAvailableGolfers => {
                        match ensure_list_events(&mut state, false) {
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
                                    .set_mode(ReplCompletionMode::PromptEvents(event_ids));
                                let response = prompt_for_events(&mut rl);
                                helper_state
                                    .borrow_mut()
                                    .set_mode(ReplCompletionMode::Repl);
                                match response {
                                    Ok(selected) => {
                                        if selected.is_empty() {
                                            println!("No events selected.");
                                        } else {
                                            println!("{}", selected.join(" "));
                                        }
                                    }
                                    Err(ReplPromptError::Interrupted) => continue,
                                    Err(ReplPromptError::Failed(err)) => {
                                        return Err(err);
                                    }
                                }
                            }
                            Err(err) => print_list_event_error(&err),
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

struct ReplState {
    cached_events: Option<Vec<(String, String)>>,
    eup_json_path: Option<PathBuf>,
    event_cache_dir: PathBuf,
    _temp_dir: TempDir,
}

impl ReplState {
    fn new(eup_json_path: Option<PathBuf>) -> Result<Self> {
        let temp_dir = TempDir::new().context("create event cache dir")?;
        let event_cache_dir = temp_dir.path().join("espn_events");
        fs::create_dir_all(&event_cache_dir)
            .with_context(|| format!("create {}", event_cache_dir.display()))?;
        Ok(Self {
            cached_events: None,
            eup_json_path,
            event_cache_dir,
            _temp_dir: temp_dir,
        })
    }
}

enum ReplPromptError {
    Interrupted,
    Failed(anyhow::Error),
}

fn list_event_error_message(err: &anyhow::Error) -> String {
    if err.is::<MalformedEspnJson>() {
        "Fetch to espn returned malformed data.".to_string()
    } else {
        format!("Fetch to espn failed: {err}")
    }
}

fn print_list_event_error(err: &anyhow::Error) {
    println!("{}", list_event_error_message(err));
}

fn find_command(name: &str) -> Option<&'static ReplCommand> {
    REPL_COMMANDS
        .iter()
        .find(|command| command.name == name || command.aliases.contains(&name))
}

fn find_subcommand(
    subcommands: &'static [ReplSubcommand],
    name: &str,
) -> Option<&'static ReplSubcommand> {
    subcommands.iter().find(|subcommand| subcommand.name == name)
}

fn print_subcommand_help(command: &ReplCommand) {
    for subcommand in command.subcommands {
        println!("{} {}", subcommand.name, subcommand.description);
    }
}

fn ensure_list_events(state: &mut ReplState, refresh: bool) -> Result<Vec<(String, String)>> {
    if state.cached_events.is_some() && !refresh {
        return Ok(state.cached_events.clone().unwrap_or_default());
    }

    let mut events = BTreeMap::new();
    for (id, name) in list_espn_events()? {
        events.insert(id, name);
    }

    if let Some(path) = state.eup_json_path.as_ref() {
        let eup_event_ids = read_eup_event_ids(path)?;
        let missing_ids: Vec<i64> = eup_event_ids
            .into_iter()
            .filter(|event_id| !events.contains_key(&event_id.to_string()))
            .collect();
        for (event_id, name) in fetch_event_names_parallel(&missing_ids, &state.event_cache_dir) {
            events.insert(event_id.to_string(), name);
        }
    }

    let cached: Vec<(String, String)> = events.into_iter().collect();
    state.cached_events = Some(cached.clone());
    Ok(cached)
}

fn read_eup_event_ids(path: &PathBuf) -> Result<Vec<i64>> {
    let contents = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let payload: Value =
        serde_json::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
    let mut ids = BTreeSet::new();
    if let Some(array) = payload.as_array() {
        for entry in array {
            if let Some(event_id) = entry.get("event").and_then(Value::as_i64) {
                ids.insert(event_id);
            }
        }
    }
    Ok(ids.into_iter().collect())
}

fn build_repl_help() -> String {
    let mut help = String::from("Commands:");
    for command in REPL_COMMANDS {
        let names = if command.aliases.is_empty() {
            command.name.to_string()
        } else {
            let mut parts = Vec::with_capacity(command.aliases.len() + 1);
            parts.push(command.name);
            parts.extend(command.aliases);
            parts.join(", ")
        };
        help.push_str("\n  ");
        help.push_str(&names);
        let padding = 22usize.saturating_sub(names.len());
        help.push_str(&" ".repeat(padding.max(2)));
        help.push_str(command.description);
    }
    help
}

fn prompt_for_events(
    rl: &mut Editor<ReplHelper, DefaultHistory>,
) -> Result<Vec<String>, ReplPromptError> {
    match rl.readline("Which events? (csv or space-separated) ") {
        Ok(line) => {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return Ok(Vec::new());
            }
            let normalized = trimmed.replace(',', " ");
            let ids = normalized
                .split_whitespace()
                .filter(|id| !id.is_empty())
                .map(str::to_string)
                .collect();
            Ok(ids)
        }
        Err(ReadlineError::Interrupted | ReadlineError::Eof) => Err(ReplPromptError::Interrupted),
        Err(err) => Err(ReplPromptError::Failed(anyhow::Error::from(err))),
    }
}

#[derive(Clone)]
enum ReplCompletionMode {
    Repl,
    PromptEvents(Vec<String>),
}

struct ReplHelperState {
    mode: ReplCompletionMode,
}

impl ReplHelperState {
    fn new() -> Self {
        Self {
            mode: ReplCompletionMode::Repl,
        }
    }

    fn set_mode(&mut self, mode: ReplCompletionMode) {
        self.mode = mode;
    }
}

struct ReplHelper {
    state: Rc<RefCell<ReplHelperState>>,
}

impl ReplHelper {
    fn new(state: Rc<RefCell<ReplHelperState>>) -> Self {
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
            ReplCompletionMode::Repl => self.complete_repl(line, pos),
            ReplCompletionMode::PromptEvents(ids) => Ok(complete_event_prompt(line, pos, &ids)),
        }
    }
}

impl ReplHelper {
    fn complete_repl(&self, line: &str, pos: usize) -> rustyline::Result<(usize, Vec<Pair>)> {
        let prefix = &line[..pos];
        let mut parts = prefix.split_whitespace();
        let first = parts.next().unwrap_or_default();
        let second = parts.next();

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
            return Ok((start, candidates));
        }

        if prefix.contains(char::is_whitespace) {
            return Ok((pos, Vec::new()));
        }

        let candidates = REPL_COMMANDS
            .iter()
            .flat_map(|command| command.aliases.iter().copied().chain([command.name]))
            .filter(|cmd| cmd.starts_with(prefix))
            .map(|cmd| Pair {
                display: cmd.to_string(),
                replacement: cmd.to_string(),
            })
            .collect();
        Ok((0, candidates))
    }
}

impl Hinter for ReplHelper {
    type Hint = String;
}

impl Highlighter for ReplHelper {}

impl Validator for ReplHelper {}

impl Helper for ReplHelper {}
