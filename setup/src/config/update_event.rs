use super::{AppMode, KvAccessConfig};
use super::cli::{Cli, FileConfig};
use crate::seed::wrangler::load_kv_namespace_id;
use anyhow::{Result, anyhow};
use std::path::PathBuf;

pub(crate) fn build_update_event_mode(cli: &Cli, file_config: &FileConfig) -> Result<AppMode> {
    if cli.one_shot || file_config.one_shot.unwrap_or(false) {
        return Err(anyhow!("--one-shot is only valid with --mode=new_event or --mode=get_event_details"));
    }

    let output_json = cli
        .output_json
        .clone()
        .or_else(|| file_config.output_json.clone());

    let kv_binding = cli
        .kv_binding
        .clone()
        .or_else(|| file_config.kv_binding.clone());
    let kv_env = cli
        .kv_env
        .clone()
        .or_else(|| file_config.kv_env.clone());

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
        let kv_env = kv_env.ok_or_else(|| anyhow!("missing --kv-env (required without --kv-binding)"))?;
        Some(load_kv_namespace_id(&wrangler_config, &kv_env)?)
    } else {
        None
    };

    Ok(AppMode::UpdateEvent {
        eup_json: cli
            .eup_json
            .clone()
            .or_else(|| file_config.eup_json.clone()),
        output_json,
        kv_access: KvAccessConfig {
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
        },
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
        && kv_env != wrangler_env {
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
        && wrangler_env != kv_flags_env {
            return Err(anyhow!(
                "--wrangler-env ({wrangler_env}) conflicts with --wrangler-kv-flag --env {kv_flags_env}"
            ));
        }
    Ok(())
}
