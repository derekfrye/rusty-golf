use crate::seed::espn_header::fetch_end_dates;
use crate::seed::eup::load_events;
use crate::seed::files::{write_auth_tokens, write_event_files};
use crate::seed::wrangler::{load_kv_namespace_id, seed_event_kv};
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

mod espn_header;
mod eup;
mod files;
mod wrangler;

#[derive(Debug)]
pub struct SeedOptions {
    pub eup_json: PathBuf,
    pub kv_env: String,
    pub event_id: Option<i64>,
    pub auth_tokens: Option<Vec<String>>,
    pub refresh_from_espn: i64,
    pub wrangler_config: PathBuf,
    pub wrangler_env: String,
    pub wrangler_kv_flags: Vec<String>,
    pub wrangler_log_dir: Option<PathBuf>,
    pub wrangler_config_dir: Option<PathBuf>,
    pub kv_binding: Option<String>,
}

/// Seed Wrangler KV entries from a EUP JSON payload.
///
/// # Errors
/// Returns an error if the input files are missing, invalid, or if the KV
/// writes fail.
pub fn seed_kv_from_eup(options: &SeedOptions) -> Result<()> {
    if options.kv_env != "dev" && options.kv_env != "prod" {
        bail!("kv_env must be 'dev' or 'prod'");
    }

    let kv_namespace_id = if options.kv_binding.is_some() {
        None
    } else {
        Some(load_kv_namespace_id(
            &options.wrangler_config,
            &options.kv_env,
        )?)
    };

    let events = load_events(&options.eup_json, options.event_id)?;
    if events.is_empty() {
        bail!("no events found to seed");
    }

    let end_dates = match fetch_end_dates() {
        Ok(map) => map,
        Err(err) => {
            eprintln!("Warning: failed to fetch ESPN end dates: {err}");
            HashMap::new()
        }
    };

    let temp_dir = TempDir::new().context("create temp dir")?;
    for event in &events {
        let end_date = event
            .end_date
            .as_deref()
            .or_else(|| end_dates.get(&event.event).map(String::as_str));
        write_event_files(event, options.refresh_from_espn, end_date, temp_dir.path())?;
        if let Some(tokens) = options.auth_tokens.as_ref() {
            let event_dir = temp_dir.path().join(event.event.to_string());
            write_auth_tokens(tokens, &event_dir)?;
        }
    }

    for event in &events {
        seed_event_kv(
            event.event,
            temp_dir.path(),
            options.kv_binding.as_deref(),
            kv_namespace_id.as_deref(),
            &options.wrangler_kv_flags,
            options.wrangler_log_dir.as_deref(),
            options.wrangler_config_dir.as_deref(),
        )?;
        println!("Seeded KV for event {}.", event.event);
    }

    println!("KV seed complete for env {}.", options.wrangler_env);
    Ok(())
}
