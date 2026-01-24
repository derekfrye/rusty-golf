use crate::seed::SeedOptions;
use anyhow::{Context, Result, anyhow};
use clap::ValueEnum;
use serde::Deserialize;

mod cli;
mod get_event_details;
mod new_event;
mod parse;
mod seed;
mod update_event;

pub use cli::Cli;

#[derive(Debug, Clone, Copy, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Seed,
    #[value(name = "new_event", alias = "setup_event", alias = "repl")]
    NewEvent,
    #[value(name = "get_event_details")]
    GetEventDetails,
    #[value(name = "update_event", alias = "edit_event")]
    UpdateEvent,
}

pub enum AppMode {
    Seed(Box<SeedOptions>),
    NewEvent {
        eup_json: Option<std::path::PathBuf>,
        output_json: Option<std::path::PathBuf>,
        output_json_stdout: bool,
        one_shot: bool,
        event_id: Option<i64>,
        golfers_by_bettor: Option<Vec<GolferByBettorInput>>,
        kv_access: Option<KvAccessConfig>,
    },
    GetEventDetails {
        eup_json: Option<std::path::PathBuf>,
        output_json: Option<std::path::PathBuf>,
        output_json_stdout: bool,
        event_ids: Option<Vec<i64>>,
    },
    UpdateEvent {
        eup_json: Option<std::path::PathBuf>,
        output_json: Option<std::path::PathBuf>,
        kv_access: KvAccessConfig,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct GolferByBettorInput {
    pub bettor: String,
    pub golfer: String,
}

pub struct KvAccessConfig {
    pub kv_binding: Option<String>,
    pub kv_namespace_id: Option<String>,
    pub wrangler_kv_flags: Vec<String>,
    pub wrangler_log_dir: Option<std::path::PathBuf>,
    pub wrangler_config_dir: Option<std::path::PathBuf>,
}

/// Load config from CLI and optional TOML file.
///
/// # Errors
/// Returns an error if required CLI values are missing, the config file is
/// unreadable or invalid, or if auth tokens are malformed.
pub fn load_config(cli: &Cli) -> Result<AppMode> {
    if cli.one_shot && cli.mode.is_none() {
        return Err(anyhow!("--one-shot requires --mode"));
    }
    let file_config = read_file_config(cli)?;
    let mode = cli
        .mode
        .or(file_config.mode)
        .ok_or_else(|| anyhow!("missing --mode"))?;
    match mode {
        Mode::Seed => seed::build_seed_mode(cli, &file_config),
        Mode::NewEvent => new_event::build_new_event_mode(cli, &file_config),
        Mode::GetEventDetails => get_event_details::build_get_event_details_mode(cli, &file_config),
        Mode::UpdateEvent => update_event::build_update_event_mode(cli, &file_config),
    }
}

fn read_file_config(cli: &Cli) -> Result<cli::FileConfig> {
    match cli.config_toml.as_ref() {
        Some(path) => {
            let contents = std::fs::read_to_string(path)
                .with_context(|| format!("read config toml {}", path.display()))?;
            toml::from_str::<cli::FileConfig>(&contents)
                .with_context(|| format!("parse config toml {}", path.display()))
        }
        None => Ok(cli::FileConfig::default()),
    }
}
