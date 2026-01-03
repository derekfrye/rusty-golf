use crate::seed::SeedOptions;
use anyhow::{Context, Result, anyhow};
use clap::ValueEnum;
use serde::Deserialize;

mod cli;
mod new_event;
mod parse;
mod seed;

pub use cli::Cli;

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
        eup_json: Option<std::path::PathBuf>,
        output_json: Option<std::path::PathBuf>,
        one_shot: bool,
        event_id: Option<i64>,
        golfers_by_bettor: Option<Vec<GolferByBettorInput>>,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct GolferByBettorInput {
    pub bettor: String,
    pub golfer: String,
}

/// Load config from CLI and optional TOML file.
///
/// # Errors
/// Returns an error if required CLI values are missing, the config file is
/// unreadable or invalid, or if auth tokens are malformed.
pub fn load_config(cli: Cli) -> Result<AppMode> {
    let file_config = read_file_config(&cli)?;
    let mode = cli
        .mode
        .or(file_config.mode)
        .ok_or_else(|| anyhow!("missing --mode"))?;
    match mode {
        Mode::Seed => seed::build_seed_mode(&cli, &file_config),
        Mode::NewEvent => new_event::build_new_event_mode(&cli, &file_config),
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
