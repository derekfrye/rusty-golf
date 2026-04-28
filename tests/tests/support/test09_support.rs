use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use rusty_golf_setup::config::GolferByBettorInput;
use serde_json::Value;

#[path = "test09_support/compare.rs"]
mod compare;
#[path = "test09_support/fixture_client.rs"]
mod fixture_client;

pub(super) use compare::assert_values_match;
pub(super) use fixture_client::FixtureEspnClient;

pub(super) fn first_two_event_ids(path: &Path) -> Result<Vec<i64>> {
    let payload = read_json(path)?;
    let array = payload
        .as_array()
        .ok_or_else(|| anyhow!("dbprefill JSON is not an array"))?;
    array
        .iter()
        .take(2)
        .map(|entry| {
            entry
                .get("event")
                .and_then(Value::as_i64)
                .ok_or_else(|| anyhow!("dbprefill entry missing event id"))
        })
        .collect()
}

pub(super) fn load_dbprefill_event(path: &Path, event_id: i64) -> Result<Value> {
    let payload = read_json(path)?;
    let array = payload
        .as_array()
        .ok_or_else(|| anyhow!("dbprefill JSON is not an array"))?;
    array
        .iter()
        .find(|entry| entry.get("event").and_then(Value::as_i64) == Some(event_id))
        .cloned()
        .ok_or_else(|| anyhow!("dbprefill entry missing event {event_id}"))
}

pub(super) fn build_one_shot_inputs_from_dbprefill(
    fixture_root: &Path,
    event_id: i64,
    expected_entry: &Value,
) -> Result<Vec<GolferByBettorInput>> {
    let golfers_by_id = load_fixture_golfers_by_id(fixture_root, event_id)?;
    let data_to_fill = load_data_to_fill(expected_entry)?;
    let event_user_player = data_to_fill
        .get("event_user_player")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("missing event_user_player for {event_id}"))?;
    event_user_player
        .iter()
        .map(|entry| golfer_input(entry, &golfers_by_id, event_id))
        .collect()
}

pub(super) fn assert_output_matches_expected(
    path: &Path,
    event_id: i64,
    expected_entry: &Value,
) -> Result<()> {
    let payload = read_json(path)?;
    let array = payload
        .as_array()
        .ok_or_else(|| anyhow!("output JSON is not an array"))?;
    let entry = array
        .iter()
        .rev()
        .find(|entry| entry.get("event").and_then(Value::as_i64) == Some(event_id))
        .ok_or_else(|| anyhow!("output JSON missing event {event_id}"))?;
    assert_event_matches_expected(entry, expected_entry)
}

fn golfer_input(
    entry: &Value,
    golfers_by_id: &HashMap<i64, String>,
    event_id: i64,
) -> Result<GolferByBettorInput> {
    let bettor = entry
        .get("bettor")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing bettor for {event_id}"))?;
    let golfer_id = entry
        .get("golfer_espn_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("missing golfer_espn_id for {event_id}"))?;
    let golfer = golfers_by_id
        .get(&golfer_id)
        .ok_or_else(|| anyhow!("missing golfer {golfer_id} in fixture for {event_id}"))?;
    Ok(GolferByBettorInput {
        bettor: bettor.to_string(),
        golfer: golfer.clone(),
    })
}

fn load_fixture_golfers_by_id(fixture_root: &Path, event_id: i64) -> Result<HashMap<i64, String>> {
    let payload = read_json(&fixture_root.join(format!("event_{event_id}.json")))?;
    let mut golfers = HashMap::new();
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
                golfers.insert(id, name.to_string());
            }
        }
    }
    Ok(golfers)
}

fn load_data_to_fill(entry: &Value) -> Result<&serde_json::Map<String, Value>> {
    entry
        .get("data_to_fill_if_event_and_year_missing")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("missing data_to_fill_if_event_and_year_missing"))
}

fn assert_event_matches_expected(actual: &Value, expected: &Value) -> Result<()> {
    let ignore_keys = ["year", "score_view_step_factor"];
    let expected_obj = expected
        .as_object()
        .ok_or_else(|| anyhow!("expected event is not an object"))?;
    let actual_obj = actual
        .as_object()
        .ok_or_else(|| anyhow!("actual event is not an object"))?;
    assert_object_keys_match(expected_obj, actual_obj, ".")?;
    for key in expected_obj.keys() {
        if ignore_keys.contains(&key.as_str()) {
            continue;
        }
        assert_values_match(&expected_obj[key], &actual_obj[key], &format!(".{key}"))?;
    }
    Ok(())
}

fn assert_object_keys_match(
    expected: &serde_json::Map<String, Value>,
    actual: &serde_json::Map<String, Value>,
    path: &str,
) -> Result<()> {
    let expected_keys: BTreeSet<&str> = expected.keys().map(String::as_str).collect();
    let actual_keys: BTreeSet<&str> = actual.keys().map(String::as_str).collect();
    if expected_keys == actual_keys {
        Ok(())
    } else {
        Err(anyhow!(
            "object keys mismatch at {path}: expected {expected_keys:?}, got {actual_keys:?}"
        ))
    }
}

fn read_json(path: &Path) -> Result<Value> {
    let contents = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&contents).with_context(|| format!("parse {}", path.display()))
}
