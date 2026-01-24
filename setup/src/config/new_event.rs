use super::cli::{Cli, FileConfig, GolfersByBettorConfig};
use super::new_event_helpers::{
    extract_env_flag, resolve_wrangler_flags, resolve_wrangler_kv_flags, validate_env_consistency,
};
use super::parse::{parse_golfers_by_bettor, parse_single_event_id};
use super::{AppMode, KvAccessConfig};
use crate::seed::wrangler::load_kv_namespace_id;
use anyhow::{Result, anyhow};
use std::path::PathBuf;

pub(crate) fn build_new_event_mode(cli: &Cli, file_config: &FileConfig) -> Result<AppMode> {
    let one_shot = if cli.one_shot {
        true
    } else {
        file_config.one_shot.unwrap_or(false)
    };
    let output_json = cli
        .output_json
        .clone()
        .or_else(|| file_config.output_json.clone());
    let output_json_stdout =
        (cli.output_json_stdout || file_config.output_json_stdout.unwrap_or(false)) && one_shot;
    let event_id_input = resolve_event_id_input(cli, file_config);
    let event_id = event_id_input
        .as_deref()
        .map(parse_single_event_id)
        .transpose()?;
    let golfers_by_bettor = resolve_golfers_by_bettor(cli, file_config)?;
    let kv_access = resolve_kv_access(cli, file_config)?;

    if one_shot {
        if event_id.is_none() {
            return Err(anyhow!("missing --event-id for --one-shot"));
        }
        if output_json.is_none() && !output_json_stdout {
            return Err(anyhow!(
                "missing --output-json or --output-json-stdout for --one-shot"
            ));
        }
        if golfers_by_bettor.is_none() {
            return Err(anyhow!("missing --golfers-by-bettor for --one-shot"));
        }
    }

    Ok(AppMode::NewEvent {
        eup_json: cli
            .eup_json
            .clone()
            .or_else(|| file_config.eup_json.clone()),
        output_json,
        output_json_stdout,
        one_shot,
        event_id,
        golfers_by_bettor,
        kv_access,
    })
}

fn resolve_golfers_by_bettor(
    cli: &Cli,
    file_config: &FileConfig,
) -> Result<Option<Vec<super::GolferByBettorInput>>> {
    if let Some(value) = cli.golfers_by_bettor.as_ref() {
        return Ok(Some(parse_golfers_by_bettor(value)?));
    }
    if let Some(value) = file_config.golfers_by_bettor.as_ref() {
        let entries = match value {
            GolfersByBettorConfig::Json(raw) => parse_golfers_by_bettor(raw)?,
            GolfersByBettorConfig::Entries(entries) => entries.clone(),
        };
        return Ok(Some(entries));
    }
    Ok(None)
}

fn resolve_event_id_input(cli: &Cli, file_config: &FileConfig) -> Option<String> {
    cli.event_id.clone().or_else(|| {
        file_config
            .event_id
            .as_ref()
            .map(super::cli::EventIdConfig::as_string)
    })
}

fn resolve_kv_access(cli: &Cli, file_config: &FileConfig) -> Result<Option<KvAccessConfig>> {
    let kv_binding = cli
        .kv_binding
        .clone()
        .or_else(|| file_config.kv_binding.clone());
    let kv_env = cli.kv_env.clone().or_else(|| file_config.kv_env.clone());
    if kv_binding.is_none() && kv_env.is_none() {
        return Ok(None);
    }

    let wrangler_config = cli
        .wrangler_config
        .clone()
        .or_else(|| file_config.wrangler_config.clone())
        .unwrap_or_else(|| PathBuf::from("serverless/wrangler.toml"));
    let wrangler_env_explicit = cli
        .wrangler_env
        .clone()
        .or_else(|| file_config.wrangler_env.clone());
    let wrangler_env = wrangler_env_explicit
        .clone()
        .or_else(|| kv_env.clone())
        .unwrap_or_else(|| "dev".to_string());
    let wrangler_flags = resolve_wrangler_flags(cli, file_config, &wrangler_config);
    let wrangler_kv_flags =
        resolve_wrangler_kv_flags(cli, file_config, &wrangler_flags, &wrangler_env);
    validate_env_consistency(
        kv_env.as_deref(),
        wrangler_env_explicit.as_deref(),
        extract_env_flag(&wrangler_kv_flags).as_deref(),
        kv_binding.as_deref(),
    )?;

    let kv_namespace_id = if kv_binding.is_none() {
        let kv_env =
            kv_env.ok_or_else(|| anyhow!("missing --kv-env (required without --kv-binding)"))?;
        Some(load_kv_namespace_id(&wrangler_config, &kv_env)?)
    } else {
        None
    };

    Ok(Some(KvAccessConfig {
        kv_binding,
        kv_namespace_id,
        wrangler_kv_flags,
        wrangler_log_dir: cli
            .wrangler_log_dir
            .clone()
            .or_else(|| file_config.wrangler_log_dir.clone()),
        wrangler_config_dir: cli
            .wrangler_config_dir
            .clone()
            .or_else(|| file_config.wrangler_config_dir.clone()),
    }))
}
