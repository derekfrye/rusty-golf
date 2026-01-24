use super::ReplState;
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::process::Command;

pub(super) fn kv_key_get(state: &ReplState, key: &str) -> Result<Option<String>> {
    let Some(kv_access) = state.kv_access.as_ref() else {
        return Ok(None);
    };
    let mut command = Command::new("wrangler");
    command.arg("kv").arg("key").arg("get").args(&kv_access.wrangler_kv_flags);
    if let Some(binding) = kv_access.kv_binding.as_deref() {
        command.arg("--binding").arg(binding);
    } else if let Some(id) = kv_access.kv_namespace_id.as_deref() {
        command.arg("--namespace-id").arg(id);
    } else {
        bail!("missing kv binding or namespace id");
    }
    command.arg(key);

    if let Some(dir) = kv_access.wrangler_log_dir.as_ref() {
        command.env("WRANGLER_LOG_DIR", dir);
    }
    if let Some(dir) = kv_access.wrangler_config_dir.as_ref() {
        command.env("XDG_CONFIG_HOME", dir);
    }

    let output = command.output().context("run wrangler kv key get")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("404") || stderr.contains("Not Found") {
            return Ok(None);
        }
        bail!("wrangler kv key get failed: {stderr}");
    }

    let stdout = String::from_utf8(output.stdout).context("parse wrangler kv output")?;
    let trimmed = stdout.trim();
    if trimmed.is_empty()
        || trimmed == "null"
        || trimmed.eq_ignore_ascii_case("value not found")
    {
        return Ok(None);
    }
    Ok(Some(trimmed.to_string()))
}

pub(super) fn kv_list_keys(state: &ReplState, prefix: &str) -> Result<Option<Vec<String>>> {
    let Some(kv_access) = state.kv_access.as_ref() else {
        return Ok(None);
    };
    let mut command = Command::new("wrangler");
    command
        .arg("kv")
        .arg("key")
        .arg("list")
        .args(&kv_access.wrangler_kv_flags)
        .arg("--prefix")
        .arg(prefix);
    if let Some(binding) = kv_access.kv_binding.as_deref() {
        command.arg("--binding").arg(binding);
    } else if let Some(id) = kv_access.kv_namespace_id.as_deref() {
        command.arg("--namespace-id").arg(id);
    } else {
        bail!("missing kv binding or namespace id");
    }

    if let Some(dir) = kv_access.wrangler_log_dir.as_ref() {
        command.env("WRANGLER_LOG_DIR", dir);
    }
    if let Some(dir) = kv_access.wrangler_config_dir.as_ref() {
        command.env("XDG_CONFIG_HOME", dir);
    }

    let output = command.output().context("run wrangler kv key list")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("wrangler kv key list failed: {stderr}");
    }

    let stdout = String::from_utf8(output.stdout).context("parse wrangler kv output")?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(Some(Vec::new()));
    }

    if trimmed.starts_with('[') || trimmed.starts_with('{') {
        let value: Value = serde_json::from_str(trimmed)
            .context("parse wrangler kv key list json")?;
        let mut keys = Vec::new();
        match value {
            Value::Array(items) => {
                for item in items {
                    if let Some(name) = item.get("name").and_then(Value::as_str) {
                        keys.push(name.to_string());
                    }
                }
            }
            Value::Object(map) => {
                if let Some(items) = map.get("result").and_then(Value::as_array) {
                    for item in items {
                        if let Some(name) = item.get("name").and_then(Value::as_str) {
                            keys.push(name.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
        return Ok(Some(keys));
    }

    let mut keys = Vec::new();
    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("KEY") {
            continue;
        }
        if let Some((key, _)) = line.split_once(' ') {
            keys.push(key.to_string());
        } else {
            keys.push(line.to_string());
        }
    }
    Ok(Some(keys))
}

pub(super) fn parse_event_id_from_key(key: &str) -> Option<i64> {
    let trimmed = key.strip_prefix("event:")?;
    let (id_part, _) = trimmed.split_once(':')?;
    id_part.parse::<i64>().ok()
}
