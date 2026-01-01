use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct WranglerConfig {
    env: Option<HashMap<String, WranglerEnv>>,
}

#[derive(Debug, Deserialize)]
struct WranglerEnv {
    kv_namespaces: Option<Vec<KvNamespace>>,
}

#[derive(Debug, Deserialize)]
struct KvNamespace {
    id: Option<String>,
}

pub(crate) fn load_kv_namespace_id(config_path: &Path, kv_env: &str) -> Result<String> {
    let contents = fs::read_to_string(config_path)
        .with_context(|| format!("read wrangler config {}", config_path.display()))?;
    let config: WranglerConfig = toml::from_str(&contents)
        .with_context(|| format!("parse wrangler config {}", config_path.display()))?;

    let envs = config
        .env
        .ok_or_else(|| anyhow!("no env section in {}", config_path.display()))?;
    let env = envs
        .get(kv_env)
        .ok_or_else(|| anyhow!("no env '{}' in {}", kv_env, config_path.display()))?;
    let namespaces = env.kv_namespaces.as_ref().ok_or_else(|| {
        anyhow!(
            "no kv_namespaces found for env '{}' in {}",
            kv_env,
            config_path.display()
        )
    })?;
    let namespace = namespaces
        .first()
        .ok_or_else(|| anyhow!("kv_namespaces empty for env '{kv_env}'"))?;
    let id = namespace.id.as_ref().ok_or_else(|| {
        anyhow!(
            "missing kv_namespaces[0].id for env '{}' in {}",
            kv_env,
            config_path.display()
        )
    })?;
    Ok(id.clone())
}

pub(crate) fn seed_event_kv(
    event_id: i64,
    root: &Path,
    kv_binding: Option<&str>,
    namespace_id: Option<&str>,
    wrangler_kv_flags: &[String],
    wrangler_log_dir: Option<&Path>,
    wrangler_config_dir: Option<&Path>,
) -> Result<()> {
    let event_dir = root.join(event_id.to_string());
    let mut entries = vec![
        (
            format!("event:{event_id}:details"),
            event_dir.join("event_details.json"),
        ),
        (
            format!("event:{event_id}:golfers"),
            event_dir.join("golfers.json"),
        ),
        (
            format!("event:{event_id}:player_factors"),
            event_dir.join("player_factors.json"),
        ),
        (
            format!("event:{event_id}:details:seeded_at"),
            event_dir.join("seeded_at.json"),
        ),
        (
            format!("event:{event_id}:golfers:seeded_at"),
            event_dir.join("seeded_at.json"),
        ),
        (
            format!("event:{event_id}:player_factors:seeded_at"),
            event_dir.join("seeded_at.json"),
        ),
    ];
    let auth_tokens_path = event_dir.join("auth_tokens.json");
    if auth_tokens_path.is_file() {
        entries.push((format!("event:{event_id}:auth_tokens"), auth_tokens_path));
    }

    for (key, path) in entries {
        let mut command = Command::new("wrangler");
        command
            .arg("kv")
            .arg("key")
            .arg("put")
            .args(wrangler_kv_flags);

        if let Some(binding) = kv_binding {
            command.arg("--binding").arg(binding);
        } else if let Some(id) = namespace_id {
            command.arg("--namespace-id").arg(id);
        } else {
            bail!("missing kv binding or namespace id");
        }

        command.arg(key).arg("--path").arg(path);

        if let Some(dir) = wrangler_log_dir {
            command.env("WRANGLER_LOG_DIR", dir);
        }
        if let Some(dir) = wrangler_config_dir {
            command.env("XDG_CONFIG_HOME", dir);
        }

        let status = command.status().context("run wrangler kv key put")?;
        if !status.success() {
            bail!("wrangler failed with status {status}");
        }
    }

    Ok(())
}
