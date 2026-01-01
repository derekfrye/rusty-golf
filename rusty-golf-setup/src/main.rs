use anyhow::{anyhow, Context, Result};
use clap::{Parser, ValueEnum};
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::Helper;
use rustyline::validate::Validator;
use rustyline::Editor;
use rusty_golf_setup::{seed_kv_from_eup, SeedOptions};
use serde::Deserialize;
use serde_json::Value;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Seed,
    #[value(name = "new_event")]
    NewEvent,
}

enum AppMode {
    Seed(Box<SeedOptions>),
    NewEvent { eup_json: Option<PathBuf> },
}

const ESPN_SCOREBOARD_URL: &str = "https://site.web.api.espn.com/apis/v2/scoreboard/header?sport=golf&league=pga&region=us&lang=en&contentorigin=espn";
const ESPN_EVENT_URL_PREFIX: &str =
    "https://site.web.api.espn.com/apis/site/v2/sports/golf/pga/leaderboard/players?region=us&lang=en&event=";

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

#[derive(Parser, Debug)]
#[command(about = "Seed Wrangler KV entries from a db_prefill-style eup.json")]
struct Cli {
    #[arg(long, value_enum)]
    mode: Option<Mode>,
    #[arg(long)]
    config_toml: Option<PathBuf>,
    #[arg(long)]
    eup_json: Option<PathBuf>,
    #[arg(long)]
    kv_env: Option<String>,
    #[arg(long)]
    kv_binding: Option<String>,
    #[arg(long)]
    auth_tokens: Option<String>,
    #[arg(long)]
    event_id: Option<i64>,
    #[arg(long)]
    refresh_from_espn: Option<i64>,
    #[arg(long)]
    wrangler_config: Option<PathBuf>,
    #[arg(long)]
    wrangler_env: Option<String>,
    #[arg(long)]
    wrangler_flag: Vec<String>,
    #[arg(long)]
    wrangler_kv_flag: Vec<String>,
    #[arg(long)]
    wrangler_log_dir: Option<PathBuf>,
    #[arg(long)]
    wrangler_config_dir: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    mode: Option<Mode>,
    eup_json: Option<PathBuf>,
    kv_env: Option<String>,
    kv_binding: Option<String>,
    auth_tokens: Option<String>,
    event_id: Option<i64>,
    refresh_from_espn: Option<i64>,
    wrangler_config: Option<PathBuf>,
    wrangler_env: Option<String>,
    wrangler_flags: Option<Vec<String>>,
    wrangler_kv_flags: Option<Vec<String>>,
    wrangler_log_dir: Option<PathBuf>,
    wrangler_config_dir: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match load_config(cli)? {
        AppMode::Seed(config) => seed_kv_from_eup(&config),
        AppMode::NewEvent { eup_json } => run_new_event_repl(eup_json),
    }
}

fn load_config(cli: Cli) -> Result<AppMode> {
    let file_config = match cli.config_toml.as_ref() {
        Some(path) => {
            let contents = fs::read_to_string(path)
                .with_context(|| format!("read config toml {}", path.display()))?;
            toml::from_str::<FileConfig>(&contents)
                .with_context(|| format!("parse config toml {}", path.display()))?
        }
        None => FileConfig::default(),
    };

    let mode = cli
        .mode
        .or(file_config.mode)
        .ok_or_else(|| anyhow!("missing --mode"))?;
    match mode {
        Mode::Seed => {
            let eup_json = cli
                .eup_json
                .or(file_config.eup_json)
                .ok_or_else(|| anyhow!("missing --eup-json"))?;
            let kv_env = cli
                .kv_env
                .or(file_config.kv_env)
                .ok_or_else(|| anyhow!("missing --kv-env"))?;

            let event_id = cli.event_id.or(file_config.event_id);
            let auth_tokens = match cli.auth_tokens.or(file_config.auth_tokens) {
                Some(value) => Some(parse_auth_tokens(&value)?),
                None => None,
            };
            let refresh_from_espn = cli
                .refresh_from_espn
                .or(file_config.refresh_from_espn)
                .unwrap_or(1);

            let wrangler_config = cli
                .wrangler_config
                .or(file_config.wrangler_config)
                .unwrap_or_else(|| PathBuf::from("rusty-golf-serverless/wrangler.toml"));
            let wrangler_env = cli
                .wrangler_env
                .or(file_config.wrangler_env)
                .unwrap_or_else(|| "dev".to_string());

            let wrangler_flags = if !cli.wrangler_flag.is_empty() {
                cli.wrangler_flag
            } else if let Some(flags) = file_config.wrangler_flags {
                flags
            } else {
                vec![
                    "--config".to_string(),
                    wrangler_config.display().to_string(),
                    "--remote".to_string(),
                    "--preview".to_string(),
                    "false".to_string(),
                ]
            };

            let wrangler_kv_flags = if !cli.wrangler_kv_flag.is_empty() {
                cli.wrangler_kv_flag
            } else if let Some(flags) = file_config.wrangler_kv_flags {
                flags
            } else {
                let mut flags = wrangler_flags.clone();
                flags.push("--env".to_string());
                flags.push(wrangler_env.clone());
                flags
            };

            Ok(AppMode::Seed(Box::new(SeedOptions {
                eup_json,
                kv_env,
                kv_binding: cli.kv_binding.or(file_config.kv_binding),
                auth_tokens,
                event_id,
                refresh_from_espn,
                wrangler_config,
                wrangler_env,
                wrangler_kv_flags,
                wrangler_log_dir: cli.wrangler_log_dir.or(file_config.wrangler_log_dir),
                wrangler_config_dir: cli
                    .wrangler_config_dir
                    .or(file_config.wrangler_config_dir),
            })))
        }
        Mode::NewEvent => Ok(AppMode::NewEvent {
            eup_json: cli.eup_json.or(file_config.eup_json),
        }),
    }
}

fn parse_auth_tokens(value: &str) -> Result<Vec<String>> {
    let tokens: Vec<String> = value
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect();
    if tokens.is_empty() {
        return Err(anyhow!("auth tokens list is empty"));
    }
    for token in &tokens {
        if token.chars().count() < 8 {
            return Err(anyhow!("auth token must be at least 8 characters"));
        }
        if token.chars().any(char::is_control) {
            return Err(anyhow!("auth token contains non-printable characters"));
        }
    }
    Ok(tokens)
}

fn run_new_event_repl(eup_json: Option<PathBuf>) -> Result<()> {
    let help_text = build_repl_help();
    println!("Entering new_event mode. Press Ctrl-C or Ctrl-D to quit.");
    let mut rl = Editor::<ReplHelper, DefaultHistory>::new().context("init repl")?;
    rl.set_helper(Some(ReplHelper));
    let mut state = ReplState::new(eup_json);
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
                        let subcommand = subcommand_token.and_then(|token| {
                            find_subcommand(command.subcommands, token)
                        });
                        if let Some(token) = subcommand_token
                            && subcommand.is_none()
                        {
                            println!("Unknown subcommand: {}", token);
                            print_subcommand_help(command);
                            continue;
                        }
                        if matches!(
                            subcommand.map(|sub| sub.id),
                            Some(SubcommandId::Help)
                        ) {
                            print_subcommand_help(command);
                            continue;
                        }
                        let refresh = matches!(
                            subcommand.map(|sub| sub.id),
                            Some(SubcommandId::Refresh)
                        );
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
                                match prompt_for_events(&mut rl) {
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

#[derive(Debug)]
struct MalformedEspnJson;

impl fmt::Display for MalformedEspnJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "malformed ESPN JSON")
    }
}

impl std::error::Error for MalformedEspnJson {}

#[derive(Default)]
struct ReplState {
    cached_events: Option<Vec<(String, String)>>,
    eup_json_path: Option<PathBuf>,
}

impl ReplState {
    fn new(eup_json_path: Option<PathBuf>) -> Self {
        Self {
            cached_events: None,
            eup_json_path,
        }
    }
}

enum ReplPromptError {
    Interrupted,
    Failed(anyhow::Error),
}

fn list_espn_events() -> Result<Vec<(String, String)>> {
    let response = reqwest::blocking::get(ESPN_SCOREBOARD_URL)
        .context("fetch ESPN events")?
        .text()
        .context("read ESPN response body")?;
    let payload: Value = serde_json::from_str(&response)
        .map_err(|_| anyhow::Error::new(MalformedEspnJson))?;
    Ok(extract_espn_events(&payload))
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
        for (event_id, name) in fetch_event_names_parallel(&missing_ids) {
            events.insert(event_id.to_string(), name);
        }
    }

    let cached: Vec<(String, String)> = events.into_iter().collect();
    state.cached_events = Some(cached.clone());
    Ok(cached)
}

fn read_eup_event_ids(path: &PathBuf) -> Result<Vec<i64>> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
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

fn fetch_event_name(event_id: i64) -> Result<String> {
    let url = format!("{ESPN_EVENT_URL_PREFIX}{event_id}");
    let response = reqwest::blocking::get(url)
        .context("fetch ESPN event")?
        .text()
        .context("read ESPN event response body")?;
    let payload: Value = serde_json::from_str(&response)
        .map_err(|_| anyhow::Error::new(MalformedEspnJson))?;
    let name = payload
        .get("event")
        .and_then(|event| event.get("name"))
        .and_then(Value::as_str)
        .or_else(|| payload.get("name").and_then(Value::as_str))
        .ok_or_else(|| anyhow::Error::new(MalformedEspnJson))?;
    Ok(name.to_string())
}

fn fetch_event_names_parallel(event_ids: &[i64]) -> Vec<(i64, String)> {
    if event_ids.is_empty() {
        return Vec::new();
    }
    let pool = match ThreadPoolBuilder::new().num_threads(4).build() {
        Ok(pool) => pool,
        Err(_) => {
            return event_ids
                .iter()
                .filter_map(|event_id| {
                    fetch_event_name(*event_id)
                        .ok()
                        .map(|name| (*event_id, name))
                })
                .collect();
        }
    };
    pool.install(|| {
        event_ids
            .par_iter()
            .filter_map(|event_id| {
                fetch_event_name(*event_id)
                    .ok()
                    .map(|name| (*event_id, name))
            })
            .collect()
    })
}

fn extract_espn_events(payload: &Value) -> Vec<(String, String)> {
    let mut events = Vec::new();
    let sports = payload.get("sports").and_then(Value::as_array);
    for sport in sports.into_iter().flatten() {
        let leagues = sport.get("leagues").and_then(Value::as_array);
        for league in leagues.into_iter().flatten() {
            let entries = league.get("events").and_then(Value::as_array);
            for event in entries.into_iter().flatten() {
                let id = event.get("id").and_then(Value::as_str);
                let name = event
                    .get("name")
                    .and_then(Value::as_str)
                    .or_else(|| event.get("shortName").and_then(Value::as_str));
                if let (Some(id), Some(name)) = (id, name) {
                    events.push((id.to_string(), name.to_string()));
                }
            }
        }
    }
    events
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

struct ReplHelper;

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let prefix = &line[..pos];
        let mut parts = prefix.split_whitespace();
        let first = parts.next().unwrap_or_default();
        let second = parts.next();

        if let Some(command) = find_command(first)
            && !command.subcommands.is_empty()
            && second.is_none()
            && prefix.contains(char::is_whitespace)
        {
            let sub_prefix = prefix
                .trim_start_matches(command.name)
                .trim_start();
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
