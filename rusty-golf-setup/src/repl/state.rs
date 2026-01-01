use crate::espn::{MalformedEspnJson, fetch_event_names_parallel, list_espn_events};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

pub(crate) struct ReplState {
    cached_events: Option<Vec<(String, String)>>,
    cached_bettors: Option<Vec<String>>,
    eup_json_path: Option<PathBuf>,
    event_cache_dir: PathBuf,
    _temp_dir: TempDir,
}

impl ReplState {
    pub(crate) fn new(eup_json_path: Option<PathBuf>) -> Result<Self> {
        let temp_dir = TempDir::new().context("create event cache dir")?;
        let event_cache_dir = temp_dir.path().join("espn_events");
        fs::create_dir_all(&event_cache_dir)
            .with_context(|| format!("create {}", event_cache_dir.display()))?;
        Ok(Self {
            cached_events: None,
            cached_bettors: None,
            eup_json_path,
            event_cache_dir,
            _temp_dir: temp_dir,
        })
    }
}

pub(crate) fn list_event_error_message(err: &anyhow::Error) -> String {
    if err.is::<MalformedEspnJson>() {
        "Fetch to espn returned malformed data.".to_string()
    } else {
        format!("Fetch to espn failed: {err}")
    }
}

pub(crate) fn print_list_event_error(err: &anyhow::Error) {
    println!("{}", list_event_error_message(err));
}

pub(crate) fn ensure_list_events(
    state: &mut ReplState,
    refresh: bool,
) -> Result<Vec<(String, String)>> {
    if state.cached_events.is_some() && !refresh {
        return Ok(state.cached_events.clone().unwrap_or_default());
    }

    let mut events = BTreeMap::new();
    for (id, name) in list_espn_events()? {
        events.insert(id, name);
    }

    if let Some(path) = state.eup_json_path.as_ref() {
        let eup_event_ids = read_eup_event_ids(path)?;
        let missing_ids: Vec<i64> = eup_event_ids
            .into_iter()
            .filter(|event_id| !events.contains_key(&event_id.to_string()))
            .collect();
        for (event_id, name) in fetch_event_names_parallel(&missing_ids, &state.event_cache_dir) {
            events.insert(event_id.to_string(), name);
        }
    }

    let cached: Vec<(String, String)> = events.into_iter().collect();
    state.cached_events = Some(cached.clone());
    Ok(cached)
}

pub(crate) fn ensure_list_bettors(state: &mut ReplState) -> Result<Vec<String>> {
    if let Some(cached) = state.cached_bettors.as_ref() {
        return Ok(cached.clone());
    }

    let Some(path) = state.eup_json_path.as_ref() else {
        state.cached_bettors = Some(Vec::new());
        return Ok(Vec::new());
    };

    let bettors = read_eup_bettors(path)?;
    state.cached_bettors = Some(bettors.clone());
    Ok(bettors)
}

fn read_eup_event_ids(path: &PathBuf) -> Result<Vec<i64>> {
    let contents = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let payload: Value =
        serde_json::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
    let mut ids = BTreeSet::new();
    if let Some(array) = payload.as_array() {
        for entry in array {
            if let Some(event_id) = entry.get("event").and_then(Value::as_i64) {
                ids.insert(event_id);
            }
        }
    }
    Ok(ids.into_iter().collect())
}

fn read_eup_bettors(path: &PathBuf) -> Result<Vec<String>> {
    let contents = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let payload: Value =
        serde_json::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
    let mut bettors = BTreeSet::new();
    if let Some(array) = payload.as_array() {
        for entry in array {
            if let Some(data_sets) = entry
                .get("data_to_fill_if_event_and_year_missing")
                .and_then(Value::as_array)
            {
                for data_set in data_sets {
                    if let Some(players) =
                        data_set.get("event_user_player").and_then(Value::as_array)
                    {
                        for player in players {
                            if let Some(bettor) = player.get("bettor").and_then(Value::as_str) {
                                bettors.insert(bettor.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(bettors.into_iter().collect())
}
