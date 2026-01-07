use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use rusty_golf_setup::config::GolferByBettorInput;
use rusty_golf_setup::espn::{EspnClient, MalformedEspnJson, extract_espn_events};
use rusty_golf_setup::repl::run_new_event_one_shot_with_client;
use serde_json::Value;

#[test]
fn test09_setup_oneshot() -> Result<()> {
    let fixture_root = fixture_root();
    let dbprefill_path = fixture_root
        .join("../test05_dbprefill.json")
        .canonicalize()
        .unwrap_or_else(|_| fixture_root.join("../test05_dbprefill.json"));
    let event_ids = first_two_event_ids(&dbprefill_path)?;

    let client: Arc<dyn EspnClient> = Arc::new(FixtureEspnClient::new(fixture_root.clone()));
    for event_id in event_ids {
        let expected_entry = load_dbprefill_event(&dbprefill_path, event_id)?;
        let golfers_by_bettor =
            build_one_shot_inputs_from_dbprefill(&fixture_root, event_id, &expected_entry)?;
        let output_path = output_path(event_id);
        run_new_event_one_shot_with_client(
            Some(dbprefill_path.clone()),
            &output_path,
            event_id,
            golfers_by_bettor,
            Some(Arc::clone(&client)),
        )?;
        assert_output_matches_expected(&output_path, event_id, &expected_entry)?;
        let _ = fs::remove_file(&output_path);
    }

    Ok(())
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test09")
}

fn output_path(event_id: i64) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    path.push(format!("setup_oneshot_{event_id}_{nanos}.json"));
    path
}

fn first_two_event_ids(path: &Path) -> Result<Vec<i64>> {
    let contents = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let payload: Value =
        serde_json::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
    let array = payload
        .as_array()
        .ok_or_else(|| anyhow!("dbprefill JSON is not an array"))?;
    let mut ids = Vec::new();
    for entry in array.iter().take(2) {
        let Some(id) = entry.get("event").and_then(Value::as_i64) else {
            return Err(anyhow!("dbprefill entry missing event id"));
        };
        ids.push(id);
    }
    Ok(ids)
}

fn load_dbprefill_event(path: &Path, event_id: i64) -> Result<Value> {
    let contents = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let payload: Value =
        serde_json::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
    let array = payload
        .as_array()
        .ok_or_else(|| anyhow!("dbprefill JSON is not an array"))?;
    array
        .iter()
        .find(|entry| entry.get("event").and_then(Value::as_i64) == Some(event_id))
        .cloned()
        .ok_or_else(|| anyhow!("dbprefill entry missing event {event_id}"))
}

fn build_one_shot_inputs_from_dbprefill(
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
    let mut selections = Vec::new();
    for entry in event_user_player {
        let bettor = entry
            .get("bettor")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("missing bettor for {event_id}"))?;
        let golfer_id = entry
            .get("golfer_espn_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| anyhow!("missing golfer_espn_id for {event_id}"))?;
        let golfer_name = golfers_by_id
            .get(&golfer_id)
            .ok_or_else(|| anyhow!("missing golfer {golfer_id} in fixture for {event_id}"))?;
        selections.push(GolferByBettorInput {
            bettor: bettor.to_string(),
            golfer: golfer_name.clone(),
        });
    }
    Ok(selections)
}

fn load_fixture_golfers_by_id(fixture_root: &Path, event_id: i64) -> Result<HashMap<i64, String>> {
    let path = fixture_root.join(format!("event_{event_id}.json"));
    let contents = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let payload: Value =
        serde_json::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
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

fn assert_output_matches_expected(
    path: &Path,
    event_id: i64,
    expected_entry: &Value,
) -> Result<()> {
    let contents = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let payload: Value =
        serde_json::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
    let array = payload
        .as_array()
        .ok_or_else(|| anyhow!("output JSON is not an array"))?;
    let entry = array
        .iter()
        .rev()
        .find(|entry| entry.get("event").and_then(Value::as_i64) == Some(event_id))
        .ok_or_else(|| anyhow!("output JSON missing event {event_id}"))?;
    let actual_event = entry.get("event").and_then(Value::as_i64);
    if actual_event != Some(event_id) {
        return Err(anyhow!("expected event {event_id}, got {actual_event:?}"));
    }
    assert_event_matches_expected(entry, expected_entry)
}

fn assert_event_matches_expected(actual: &Value, expected: &Value) -> Result<()> {
    let ignore_keys = ["year", "score_view_step_factor"];
    let expected_obj = expected
        .as_object()
        .ok_or_else(|| anyhow!("expected event is not an object"))?;
    let actual_obj = actual
        .as_object()
        .ok_or_else(|| anyhow!("actual event is not an object"))?;
    let expected_keys: BTreeSet<&str> = expected_obj.keys().map(String::as_str).collect();
    let actual_keys: BTreeSet<&str> = actual_obj.keys().map(String::as_str).collect();
    if expected_keys != actual_keys {
        return Err(anyhow!(
            "event keys mismatch: expected {expected_keys:?}, got {actual_keys:?}"
        ));
    }
    for key in expected_obj.keys() {
        if ignore_keys.contains(&key.as_str()) {
            continue;
        }
        let expected_value = expected_obj
            .get(key)
            .ok_or_else(|| anyhow!("missing expected key {key}"))?;
        let actual_value = actual_obj
            .get(key)
            .ok_or_else(|| anyhow!("missing actual key {key}"))?;
        assert_values_match(expected_value, actual_value, &format!(".{key}"))?;
    }
    Ok(())
}

fn assert_values_match(expected: &Value, actual: &Value, path: &str) -> Result<()> {
    match (expected, actual) {
        (Value::Object(expected_map), Value::Object(actual_map)) => {
            let expected_keys: BTreeSet<&str> = expected_map.keys().map(String::as_str).collect();
            let actual_keys: BTreeSet<&str> = actual_map.keys().map(String::as_str).collect();
            if expected_keys != actual_keys {
                return Err(anyhow!(
                    "object keys mismatch at {path}: expected {expected_keys:?}, got {actual_keys:?}"
                ));
            }
            for key in expected_map.keys() {
                let expected_value = expected_map
                    .get(key)
                    .ok_or_else(|| anyhow!("missing expected key {key} at {path}"))?;
                let actual_value = actual_map
                    .get(key)
                    .ok_or_else(|| anyhow!("missing actual key {key} at {path}"))?;
                assert_values_match(expected_value, actual_value, &format!("{path}.{key}"))?;
            }
            Ok(())
        }
        (Value::Array(expected_arr), Value::Array(actual_arr)) => {
            if expected_arr.len() != actual_arr.len() {
                return Err(anyhow!(
                    "array length mismatch at {path}: expected {}, got {}",
                    expected_arr.len(),
                    actual_arr.len()
                ));
            }
            let mut expected_items: Vec<String> =
                expected_arr.iter().map(canonicalize_value).collect();
            let mut actual_items: Vec<String> = actual_arr.iter().map(canonicalize_value).collect();
            expected_items.sort();
            actual_items.sort();
            if expected_items != actual_items {
                return Err(anyhow!("array values mismatch at {path}"));
            }
            Ok(())
        }
        _ => {
            if expected != actual {
                return Err(anyhow!(
                    "value mismatch at {path}: expected {expected}, got {actual}"
                ));
            }
            Ok(())
        }
    }
}

fn canonicalize_value(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            serde_json::to_string(value).unwrap_or_default()
        }
        Value::Array(items) => {
            let mut normalized: Vec<String> = items.iter().map(canonicalize_value).collect();
            normalized.sort();
            format!("[{}]", normalized.join(","))
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .into_iter()
                .map(|key| {
                    let value = map.get(key).unwrap_or(&Value::Null);
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap_or_default(),
                        canonicalize_value(value)
                    )
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
    }
}

struct FixtureEspnClient {
    root: PathBuf,
}

impl FixtureEspnClient {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn scoreboard_path(&self) -> PathBuf {
        self.root.join("scoreboard.json")
    }

    fn event_path(&self, event_id: i64) -> PathBuf {
        self.root.join(format!("event_{event_id}.json"))
    }
}

impl EspnClient for FixtureEspnClient {
    fn list_events(&self) -> Result<Vec<(String, String)>> {
        let contents = fs::read_to_string(self.scoreboard_path())
            .with_context(|| format!("read {}", self.scoreboard_path().display()))?;
        let payload: Value = serde_json::from_str(&contents)
            .with_context(|| format!("parse {}", self.scoreboard_path().display()))?;
        Ok(extract_espn_events(&payload))
    }

    fn fetch_event_name(&self, event_id: i64, cache_dir: &Path) -> Result<String> {
        let payload = self.fetch_event_json_cached(event_id, cache_dir)?;
        let name = payload
            .get("event")
            .and_then(|event| event.get("name"))
            .and_then(Value::as_str)
            .or_else(|| payload.get("name").and_then(Value::as_str))
            .ok_or_else(|| anyhow!(MalformedEspnJson))?;
        Ok(name.to_string())
    }

    fn fetch_event_names_parallel(
        &self,
        event_ids: &[i64],
        cache_dir: &Path,
        progress: Option<&indicatif::ProgressBar>,
    ) -> Vec<(i64, String)> {
        event_ids
            .iter()
            .filter_map(|event_id| {
                let fetched = self
                    .fetch_event_name(*event_id, cache_dir)
                    .ok()
                    .map(|name| (*event_id, name));
                if let Some(bar) = progress {
                    bar.inc(1);
                }
                fetched
            })
            .collect()
    }

    fn fetch_event_json_cached(&self, event_id: i64, cache_dir: &Path) -> Result<Value> {
        let cache_path = cache_dir.join(format!("{event_id}.json"));
        if cache_path.is_file() {
            let contents = fs::read_to_string(&cache_path)
                .with_context(|| format!("read {}", cache_path.display()))?;
            let payload: Value = serde_json::from_str(&contents)
                .with_context(|| format!("parse {}", cache_path.display()))?;
            return Ok(payload);
        }
        let source_path = self.event_path(event_id);
        let contents = fs::read_to_string(&source_path)
            .with_context(|| format!("read {}", source_path.display()))?;
        fs::write(&cache_path, &contents)
            .with_context(|| format!("write {}", cache_path.display()))?;
        let payload: Value = serde_json::from_str(&contents)
            .with_context(|| format!("parse {}", source_path.display()))?;
        Ok(payload)
    }
}
