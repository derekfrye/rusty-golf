use crate::seed::SeedOptions;
use anyhow::{Context, Result, anyhow};
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Seed,
    #[value(name = "new_event")]
    NewEvent,
}

pub enum AppMode {
    Seed(Box<SeedOptions>),
    NewEvent {
        eup_json: Option<PathBuf>,
        output_json: Option<PathBuf>,
    },
}

#[derive(Parser, Debug)]
#[command(about = "Seed Wrangler KV entries from a db_prefill-style eup.json")]
pub struct Cli {
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
    #[arg(long)]
    output_json: Option<PathBuf>,
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
    output_json: Option<PathBuf>,
}

/// Load config from CLI and optional TOML file.
///
/// # Errors
/// Returns an error if required CLI values are missing, the config file is
/// unreadable or invalid, or if auth tokens are malformed.
pub fn load_config(cli: Cli) -> Result<AppMode> {
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
                wrangler_config_dir: cli.wrangler_config_dir.or(file_config.wrangler_config_dir),
            })))
        }
        Mode::NewEvent => Ok(AppMode::NewEvent {
            eup_json: cli.eup_json.or(file_config.eup_json),
            output_json: cli.output_json.or(file_config.output_json),
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
