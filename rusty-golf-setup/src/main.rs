use anyhow::{anyhow, Context, Result};
use clap::{Parser, ValueEnum};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use rusty_golf_setup::{seed_kv_from_eup, SeedOptions};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Seed,
    #[value(rename = "new_event")]
    NewEvent,
}

enum AppMode {
    Seed(SeedOptions),
    NewEvent,
}

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
        AppMode::NewEvent => run_new_event_repl(),
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

    let mode = cli.mode.or(file_config.mode).unwrap_or(Mode::Seed);
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

            Ok(AppMode::Seed(SeedOptions {
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
            }))
        }
        Mode::NewEvent => Ok(AppMode::NewEvent),
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

fn run_new_event_repl() -> Result<()> {
    println!("Entering new_event mode. Press Ctrl-C or Ctrl-D to quit.");
    let mut rl = DefaultEditor::new().context("init repl")?;
    loop {
        match rl.readline("new_event> ") {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                rl.add_history_entry(input)?;
                if matches!(input, "exit" | "quit") {
                    break;
                }
                println!("(unhandled) {input}");
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(err) => return Err(err).context("read repl input"),
        }
    }
    Ok(())
}
