use super::ReplState;
use crate::espn::EspnClient;
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn has_cached_events(state: &ReplState) -> Result<bool> {
    let mut entries = fs::read_dir(&state.event_cache_dir)
        .with_context(|| format!("read {}", state.event_cache_dir.display()))?;
    Ok(entries.any(|entry| {
        entry
            .ok()
            .and_then(|entry| entry.path().extension().map(|ext| ext == "json"))
            .unwrap_or(false)
    }))
}

pub(crate) fn load_cached_golfers(state: &ReplState) -> Result<Vec<(String, i64)>> {
    let entries = fs::read_dir(&state.event_cache_dir)
        .with_context(|| format!("read {}", state.event_cache_dir.display()))?;
    let mut golfers = BTreeMap::new();
    for entry in entries {
        let path = entry
            .with_context(|| format!("read {}", state.event_cache_dir.display()))?
            .path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let contents =
            fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let payload: Value =
            serde_json::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
        if let Some(leaderboard) = payload.get("leaderboard").and_then(Value::as_array) {
            for entry in leaderboard {
                let name = entry
                    .get("displayName")
                    .and_then(Value::as_str)
                    .or_else(|| entry.get("fullName").and_then(Value::as_str));
                let id = entry.get("id").and_then(Value::as_str);
                if let (Some(name), Some(id)) = (name, id)
                    && let Ok(id) = id.parse::<i64>()
                {
                    golfers.entry(name.to_string()).or_insert(id);
                }
            }
        }
    }
    Ok(golfers.into_iter().collect())
}

pub(crate) fn load_event_golfers(state: &ReplState, event_id: &str) -> Result<Vec<(String, i64)>> {
    let cache_path = state.event_cache_dir.join(format!("{event_id}.json"));
    let contents = fs::read_to_string(&cache_path)
        .with_context(|| format!("read {}", cache_path.display()))?;
    let payload: Value = serde_json::from_str(&contents)
        .with_context(|| format!("parse {}", cache_path.display()))?;
    let mut golfers = Vec::new();
    if let Some(leaderboard) = payload.get("leaderboard").and_then(Value::as_array) {
        for entry in leaderboard {
            let name = entry
                .get("displayName")
                .and_then(Value::as_str)
                .or_else(|| entry.get("fullName").and_then(Value::as_str));
            let id = entry.get("id").and_then(Value::as_str);
            if let (Some(name), Some(id)) = (name, id)
                && let Ok(id) = id.parse::<i64>()
            {
                golfers.push((name.to_string(), id));
            }
        }
    }
    Ok(golfers)
}

pub(crate) fn warm_event_cache(
    events: &[(String, String)],
    cache_dir: &Path,
    espn: &Arc<dyn EspnClient>,
) -> Result<()> {
    let missing_ids: Vec<i64> = events
        .iter()
        .filter_map(|(event_id, _)| {
            let cache_path = cache_dir.join(format!("{event_id}.json"));
            if cache_path.is_file() {
                return None;
            }
            event_id.parse::<i64>().ok()
        })
        .collect();
    if missing_ids.is_empty() {
        return Ok(());
    }

    let overall_style =
        ProgressStyle::with_template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar());
    let overall = ProgressBar::new(missing_ids.len() as u64);
    overall.set_style(overall_style);
    overall.set_message("Warming event cache");
    overall.enable_steady_tick(Duration::from_millis(120));

    for event_id in missing_ids {
        let _ = espn.fetch_event_name(event_id, cache_dir)?;
        overall.inc(1);
    }
    overall.finish_and_clear();
    Ok(())
}
