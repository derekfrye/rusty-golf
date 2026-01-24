use super::AppMode;
use super::cli::Cli;
use super::cli::FileConfig;
use super::parse::{parse_auth_tokens, parse_single_event_id};
use crate::seed::SeedOptions;
use anyhow::{Result, anyhow};
use std::path::PathBuf;

pub(crate) fn build_seed_mode(cli: &Cli, file_config: &FileConfig) -> Result<AppMode> {
    if cli.one_shot || file_config.one_shot.unwrap_or(false) {
        return Err(anyhow!(
            "--one-shot is only valid with --mode=new_event or --mode=get_event_details"
        ));
    }
    let eup_json = cli
        .eup_json
        .clone()
        .or_else(|| file_config.eup_json.clone())
        .ok_or_else(|| anyhow!("missing --eup-json"))?;
    let kv_env = cli
        .kv_env
        .clone()
        .or_else(|| file_config.kv_env.clone())
        .ok_or_else(|| anyhow!("missing --kv-env"))?;

    let event_id_input = resolve_event_id_input(cli, file_config);
    let event_id = event_id_input
        .as_deref()
        .map(parse_single_event_id)
        .transpose()?;
    let auth_tokens = match cli
        .auth_tokens
        .as_deref()
        .or(file_config.auth_tokens.as_deref())
    {
        Some(value) => Some(parse_auth_tokens(value)?),
        None => None,
    };
    let refresh_from_espn = cli
        .refresh_from_espn
        .or(file_config.refresh_from_espn)
        .unwrap_or(1);

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
        .unwrap_or_else(|| kv_env.clone());
    let wrangler_flags = resolve_wrangler_flags(cli, file_config, &wrangler_config);
    let wrangler_kv_flags =
        resolve_wrangler_kv_flags(cli, file_config, &wrangler_flags, &wrangler_env);
    validate_env_consistency(
        Some(&kv_env),
        wrangler_env_explicit.as_deref(),
        extract_env_flag(&wrangler_kv_flags).as_deref(),
        cli.kv_binding
            .as_deref()
            .or(file_config.kv_binding.as_deref()),
    )?;

    Ok(AppMode::Seed(Box::new(SeedOptions {
        eup_json,
        kv_env,
        kv_binding: cli
            .kv_binding
            .clone()
            .or_else(|| file_config.kv_binding.clone()),
        auth_tokens,
        event_id,
        refresh_from_espn,
        wrangler_config,
        wrangler_env,
        wrangler_kv_flags,
        wrangler_log_dir: cli
            .wrangler_log_dir
            .clone()
            .or_else(|| file_config.wrangler_log_dir.clone()),
        wrangler_config_dir: cli
            .wrangler_config_dir
            .clone()
            .or_else(|| file_config.wrangler_config_dir.clone()),
    })))
}

fn resolve_event_id_input(cli: &Cli, file_config: &FileConfig) -> Option<String> {
    cli.event_id.clone().or_else(|| {
        file_config
            .event_id
            .as_ref()
            .map(super::cli::EventIdConfig::as_string)
    })
}

fn resolve_wrangler_flags(
    cli: &Cli,
    file_config: &FileConfig,
    wrangler_config: &std::path::Path,
) -> Vec<String> {
    if !cli.wrangler_flag.is_empty() {
        return cli.wrangler_flag.clone();
    }
    if let Some(flags) = file_config.wrangler_flags.as_ref() {
        return flags.clone();
    }
    vec![
        "--config".to_string(),
        wrangler_config.display().to_string(),
        "--remote".to_string(),
        "--preview".to_string(),
        "false".to_string(),
    ]
}

fn resolve_wrangler_kv_flags(
    cli: &Cli,
    file_config: &FileConfig,
    wrangler_flags: &[String],
    wrangler_env: &str,
) -> Vec<String> {
    if !cli.wrangler_kv_flag.is_empty() {
        return cli.wrangler_kv_flag.clone();
    }
    if let Some(flags) = file_config.wrangler_kv_flags.as_ref() {
        return flags.clone();
    }
    let mut flags = wrangler_flags.to_vec();
    flags.push("--env".to_string());
    flags.push(wrangler_env.to_string());
    flags
}

fn extract_env_flag(flags: &[String]) -> Option<String> {
    flags
        .iter()
        .position(|flag| flag == "--env")
        .and_then(|idx| flags.get(idx + 1))
        .cloned()
}

fn validate_env_consistency(
    kv_env: Option<&str>,
    wrangler_env: Option<&str>,
    kv_flags_env: Option<&str>,
    kv_binding: Option<&str>,
) -> Result<()> {
    if kv_env.is_some() && kv_binding.is_some() {
        return Err(anyhow!(
            "--kv-env conflicts with --kv-binding; choose one targeting method"
        ));
    }
    if let (Some(kv_env), Some(wrangler_env)) = (kv_env, wrangler_env)
        && kv_env != wrangler_env
    {
        return Err(anyhow!(
            "--kv-env ({kv_env}) conflicts with --wrangler-env ({wrangler_env})"
        ));
    }
    if let (Some(kv_env), Some(kv_flags_env)) = (kv_env, kv_flags_env) {
        if kv_env != kv_flags_env {
            return Err(anyhow!(
                "--kv-env ({kv_env}) conflicts with --wrangler-kv-flag --env {kv_flags_env}"
            ));
        }
    } else if let (Some(wrangler_env), Some(kv_flags_env)) = (wrangler_env, kv_flags_env)
        && wrangler_env != kv_flags_env
    {
        return Err(anyhow!(
            "--wrangler-env ({wrangler_env}) conflicts with --wrangler-kv-flag --env {kv_flags_env}"
        ));
    }
    Ok(())
}
