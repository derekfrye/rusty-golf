use anyhow::{anyhow, Context, Result};
use clap::Parser;
use rusty_golf_setup::{seed_kv_from_eup, SeedOptions};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(about = "Seed Wrangler KV entries from a db_prefill-style eup.json")]
struct Cli {
    #[arg(long)]
    config_toml: Option<PathBuf>,
    #[arg(long)]
    eup_json: Option<PathBuf>,
    #[arg(long)]
    kv_env: Option<String>,
    #[arg(long)]
    kv_binding: Option<String>,
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
    eup_json: Option<PathBuf>,
    kv_env: Option<String>,
    kv_binding: Option<String>,
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
    let config = load_config(cli)?;
    seed_kv_from_eup(config)
}

fn load_config(cli: Cli) -> Result<SeedOptions> {
    let file_config = match cli.config_toml.as_ref() {
        Some(path) => {
            let contents = fs::read_to_string(path)
                .with_context(|| format!("read config toml {}", path.display()))?;
            toml::from_str::<FileConfig>(&contents)
                .with_context(|| format!("parse config toml {}", path.display()))?
        }
        None => FileConfig::default(),
    };

    let eup_json = cli
        .eup_json
        .or(file_config.eup_json)
        .ok_or_else(|| anyhow!("missing --eup-json"))?;
    let kv_env = cli
        .kv_env
        .or(file_config.kv_env)
        .ok_or_else(|| anyhow!("missing --kv-env"))?;

    let event_id = cli.event_id.or(file_config.event_id);
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

    Ok(SeedOptions {
        eup_json,
        kv_env,
        kv_binding: cli.kv_binding.or(file_config.kv_binding),
        event_id,
        refresh_from_espn,
        wrangler_config,
        wrangler_env,
        wrangler_kv_flags,
        wrangler_log_dir: cli.wrangler_log_dir.or(file_config.wrangler_log_dir),
        wrangler_config_dir: cli
            .wrangler_config_dir
            .or(file_config.wrangler_config_dir),
    })
}
