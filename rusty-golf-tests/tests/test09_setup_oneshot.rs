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
    let dbprefill_path =
        fixture_root.join("../test5_dbprefill.json").canonicalize().unwrap_or_else(|_| {
            fixture_root.join("../test5_dbprefill.json")
        });
    let event_ids = first_two_event_ids(&dbprefill_path)?;

    let client = Arc::new(FixtureEspnClient::new(fixture_root.clone()));
    for event_id in event_ids {
        let (golfers, golfers_by_bettor) = build_one_shot_inputs(&fixture_root, event_id)?;
        let output_path = output_path(event_id);
        run_new_event_one_shot_with_client(
            Some(dbprefill_path.clone()),
            output_path.clone(),
            event_id,
            golfers_by_bettor,
            Some(Arc::clone(&client)),
        )?;
        assert_output(&output_path, event_id, &golfers)?;
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
    let contents = fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    let payload: Value = serde_json::from_str(&contents)
        .with_context(|| format!("parse {}", path.display()))?;
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

fn build_one_shot_inputs(
    fixture_root: &Path,
    event_id: i64,
) -> Result<(Vec<(String, i64)>, Vec<GolferByBettorInput>)> {
    let golfers = load_fixture_golfers(fixture_root, event_id)?;
    let selected = golfers
        .iter()
        .take(2)
        .map(|(name, id)| (name.clone(), *id))
        .collect::<Vec<_>>();
    if selected.len() < 2 {
        return Err(anyhow!("not enough golfers in fixture for {event_id}"));
    }
    let golfers_by_bettor = vec![
        GolferByBettorInput {
            bettor: "alice".to_string(),
            golfer: selected[0].0.clone(),
        },
        GolferByBettorInput {
            bettor: "bob".to_string(),
            golfer: selected[1].0.clone(),
        },
    ];
    Ok((selected, golfers_by_bettor))
}

fn load_fixture_golfers(fixture_root: &Path, event_id: i64) -> Result<Vec<(String, i64)>> {
    let path = fixture_root.join(format!("event_{event_id}.json"));
    let contents = fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    let payload: Value = serde_json::from_str(&contents)
        .with_context(|| format!("parse {}", path.display()))?;
    let mut golfers = Vec::new();
    if let Some(leaderboard) = payload.get("leaderboard").and_then(Value::as_array) {
        for entry in leaderboard {
            let name = entry
                .get("displayName")
                .and_then(Value::as_str)
                .or_else(|| entry.get("fullName").and_then(Value::as_str));
            let id = entry.get("id").and_then(Value::as_str);
            if let (Some(name), Some(id)) = (name, id) {
                if let Ok(id) = id.parse::<i64>() {
                    golfers.push((name.to_string(), id));
                }
            }
        }
    }
    Ok(golfers)
}

fn assert_output(path: &Path, event_id: i64, golfers: &[(String, i64)]) -> Result<()> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    let payload: Value = serde_json::from_str(&contents)
        .with_context(|| format!("parse {}", path.display()))?;
    let array = payload
        .as_array()
        .ok_or_else(|| anyhow!("output JSON is not an array"))?;
    let entry = array
        .last()
        .ok_or_else(|| anyhow!("output JSON missing events"))?;
    let actual_event = entry.get("event").and_then(Value::as_i64);
    if actual_event != Some(event_id) {
        return Err(anyhow!("expected event {event_id}, got {actual_event:?}"));
    }
    let golfer_names: Vec<&str> = golfers.iter().map(|(name, _)| name.as_str()).collect();
    let golfers_payload = entry
        .get("data_to_fill_if_event_and_year_missing")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(|entry| entry.get("golfers"))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("missing golfers payload"))?;
    for golfer_name in golfer_names {
        let found = golfers_payload.iter().any(|entry| {
            entry
                .get("name")
                .and_then(Value::as_str)
                .map(|name| name == golfer_name)
                .unwrap_or(false)
        });
        if !found {
            return Err(anyhow!("missing golfer {golfer_name} in output"));
        }
    }
    Ok(())
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
