use super::ReplState;
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;

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

pub(crate) fn persist_bettors_selection(state: &ReplState, bettors: &[String]) -> Result<()> {
    let mut contents = bettors.join("\n");
    if !contents.is_empty() {
        contents.push('\n');
    }
    fs::write(&state.bettors_selection_path, contents)
        .with_context(|| format!("write {}", state.bettors_selection_path.display()))?;
    Ok(())
}

pub(crate) fn bettors_selection_exists(state: &ReplState) -> bool {
    state.bettors_selection_path.is_file()
}

pub(crate) fn load_bettors_selection(state: &ReplState) -> Result<Vec<String>> {
    let contents = fs::read_to_string(&state.bettors_selection_path)
        .with_context(|| format!("read {}", state.bettors_selection_path.display()))?;
    let bettors = contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    Ok(bettors)
}

fn read_eup_bettors(path: &std::path::PathBuf) -> Result<Vec<String>> {
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
