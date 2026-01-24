use super::cli::FileConfig;
use super::Cli;
use anyhow::{Result, anyhow};
use std::path::Path;

pub(super) fn resolve_wrangler_flags(
    cli: &Cli,
    file_config: &FileConfig,
    wrangler_config: &Path,
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

pub(super) fn resolve_wrangler_kv_flags(
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

pub(super) fn extract_env_flag(flags: &[String]) -> Option<String> {
    flags
        .iter()
        .position(|flag| flag == "--env")
        .and_then(|idx| flags.get(idx + 1))
        .cloned()
}

pub(super) fn validate_env_consistency(
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
