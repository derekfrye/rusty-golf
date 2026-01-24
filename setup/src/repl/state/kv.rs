use super::ReplState;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

#[derive(Deserialize)]
struct KvGolferAssignment {
    bettor_name: String,
    golfer_name: String,
    espn_id: i64,
}

#[derive(Deserialize)]
struct KvEventDetails {
    event_name: String,
}

pub(crate) fn load_current_golfers_by_bettor(
    state: &ReplState,
    event_id: i64,
) -> Result<Option<BTreeMap<String, Vec<String>>>> {
    let Some(assignments) = load_kv_golfers(state, event_id)? else {
        return Ok(None);
    };
    Ok(Some(build_golfers_by_bettor(&assignments)))
}

pub(crate) fn load_kv_bettors(
    state: &ReplState,
    event_id: i64,
) -> Result<Option<Vec<String>>> {
    let Some(assignments) = load_kv_golfers(state, event_id)? else {
        return Ok(None);
    };
    let mut bettors = BTreeSet::new();
    for entry in assignments {
        bettors.insert(entry.bettor_name);
    }
    Ok(Some(bettors.into_iter().collect()))
}

pub(crate) fn load_kv_golfers_list(
    state: &ReplState,
    event_id: i64,
) -> Result<Option<Vec<(String, i64)>>> {
    let Some(assignments) = load_kv_golfers(state, event_id)? else {
        return Ok(None);
    };
    let mut golfers = BTreeMap::new();
    for entry in assignments {
        golfers.entry(entry.golfer_name).or_insert(entry.espn_id);
    }
    Ok(Some(golfers.into_iter().collect()))
}

pub(crate) fn list_kv_events(
    state: &ReplState,
) -> Result<Option<Vec<(i64, Option<String>)>>> {
    let Some(raw_keys) = kv_list_keys(state, "event:")? else {
        return Ok(None);
    };
    let mut event_ids = BTreeSet::new();
    for key in raw_keys {
        if let Some(event_id) = parse_event_id_from_key(&key) {
            event_ids.insert(event_id);
        }
    }
    if event_ids.is_empty() {
        return Ok(Some(Vec::new()));
    }

    let mut events = Vec::new();
    for event_id in event_ids {
        let name = load_kv_event_name(state, event_id)?;
        events.push((event_id, name));
    }
    Ok(Some(events))
}

fn load_kv_event_name(state: &ReplState, event_id: i64) -> Result<Option<String>> {
    let key = format!("event:{event_id}:details");
    let Some(raw) = kv_key_get(state, &key)? else {
        return Ok(None);
    };
    let details: KvEventDetails =
        serde_json::from_str(&raw).context("parse kv event details json")?;
    Ok(Some(details.event_name))
}

fn load_kv_golfers(
    state: &ReplState,
    event_id: i64,
) -> Result<Option<Vec<KvGolferAssignment>>> {
    let key = format!("event:{event_id}:golfers");
    let Some(raw) = kv_key_get(state, &key)? else {
        return Ok(None);
    };
    let assignments: Vec<KvGolferAssignment> =
        serde_json::from_str(&raw).context("parse kv golfers json")?;
    Ok(Some(assignments))
}

fn build_golfers_by_bettor(
    assignments: &[KvGolferAssignment],
) -> BTreeMap<String, Vec<String>> {
    let mut golfers_by_bettor: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in assignments {
        golfers_by_bettor
            .entry(entry.bettor_name.clone())
            .or_default()
            .push(format!("{} ({})", entry.golfer_name, entry.espn_id));
    }
    golfers_by_bettor
}

fn kv_key_get(state: &ReplState, key: &str) -> Result<Option<String>> {
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

fn kv_list_keys(state: &ReplState, prefix: &str) -> Result<Option<Vec<String>>> {
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

fn parse_event_id_from_key(key: &str) -> Option<i64> {
    let trimmed = key.strip_prefix("event:")?;
    let (id_part, _) = trimmed.split_once(':')?;
    id_part.parse::<i64>().ok()
}
