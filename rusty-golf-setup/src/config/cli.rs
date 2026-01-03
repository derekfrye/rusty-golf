use super::{GolferByBettorInput, Mode};
use clap::Parser;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(about = "Setup rusty-golf events, incl. seeding Wrangler KV entries.")]
pub struct Cli {
    #[arg(long, value_enum)]
    pub mode: Option<Mode>,
    #[arg(long)]
    pub one_shot: bool,
    #[arg(long)]
    pub config_toml: Option<PathBuf>,
    #[arg(long)]
    pub eup_json: Option<PathBuf>,
    #[arg(long)]
    pub kv_env: Option<String>,
    #[arg(long)]
    pub kv_binding: Option<String>,
    #[arg(long)]
    pub auth_tokens: Option<String>,
    #[arg(long)]
    pub event_id: Option<i64>,
    #[arg(long)]
    pub refresh_from_espn: Option<i64>,
    #[arg(long)]
    pub wrangler_config: Option<PathBuf>,
    #[arg(long)]
    pub wrangler_env: Option<String>,
    #[arg(long)]
    pub wrangler_flag: Vec<String>,
    #[arg(long)]
    pub wrangler_kv_flag: Vec<String>,
    #[arg(long)]
    pub wrangler_log_dir: Option<PathBuf>,
    #[arg(long)]
    pub wrangler_config_dir: Option<PathBuf>,
    #[arg(long)]
    pub output_json: Option<PathBuf>,
    #[arg(long)]
    pub golfers_by_bettor: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct FileConfig {
    pub mode: Option<Mode>,
    pub eup_json: Option<PathBuf>,
    pub kv_env: Option<String>,
    pub kv_binding: Option<String>,
    pub auth_tokens: Option<String>,
    pub event_id: Option<i64>,
    pub refresh_from_espn: Option<i64>,
    pub wrangler_config: Option<PathBuf>,
    pub wrangler_env: Option<String>,
    pub wrangler_flags: Option<Vec<String>>,
    pub wrangler_kv_flags: Option<Vec<String>>,
    pub wrangler_log_dir: Option<PathBuf>,
    pub wrangler_config_dir: Option<PathBuf>,
    pub output_json: Option<PathBuf>,
    #[serde(rename = "one-shot")]
    pub one_shot: Option<bool>,
    #[serde(rename = "golfers-by-bettor")]
    pub golfers_by_bettor: Option<GolfersByBettorConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum GolfersByBettorConfig {
    Json(String),
    Entries(Vec<GolferByBettorInput>),
}
