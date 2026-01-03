use super::ReplState;
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

pub(crate) fn eup_event_exists(state: &ReplState, event_id: i64) -> Result<bool> {
    let Some(path) = state.eup_json_path.as_ref() else {
        return Ok(false);
    };
    let contents = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let payload: Value =
        serde_json::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
    if let Some(array) = payload.as_array() {
        return Ok(array
            .iter()
            .any(|entry| entry.get("event").and_then(Value::as_i64) == Some(event_id)));
    }
    Ok(false)
}

pub(crate) fn load_eup_json(state: &ReplState) -> Result<Vec<Value>> {
    let Some(path) = state.eup_json_path.as_ref() else {
        return Ok(Vec::new());
    };
    let contents = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let payload: Value =
        serde_json::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
    let Some(array) = payload.as_array() else {
        return Err(anyhow::Error::msg("eup json must be an array"));
    };
    Ok(array.to_vec())
}

pub(crate) fn read_eup_event_ids(path: &PathBuf) -> Result<Vec<i64>> {
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
