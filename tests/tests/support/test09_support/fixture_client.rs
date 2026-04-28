use anyhow::{Context, Result, anyhow};
use rusty_golf_setup::espn::{EspnClient, MalformedEspnJson, extract_espn_events};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) struct FixtureEspnClient {
    root: PathBuf,
}

impl FixtureEspnClient {
    pub(crate) fn new(root: PathBuf) -> Self {
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
        let payload = read_json(&self.scoreboard_path())?;
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
            return read_json(&cache_path);
        }
        let source_path = self.event_path(event_id);
        let contents = fs::read_to_string(&source_path)
            .with_context(|| format!("read {}", source_path.display()))?;
        fs::write(&cache_path, &contents)
            .with_context(|| format!("write {}", cache_path.display()))?;
        serde_json::from_str(&contents).with_context(|| format!("parse {}", source_path.display()))
    }

    fn fetch_scoreboard_header_cached(&self, cache_dir: &Path) -> Result<Value> {
        let cache_path = cache_dir.join("scoreboard_header.json");
        if cache_path.is_file() {
            return read_json(&cache_path);
        }
        let source_path = self.scoreboard_path();
        let contents = fs::read_to_string(&source_path)
            .with_context(|| format!("read {}", source_path.display()))?;
        fs::write(&cache_path, &contents)
            .with_context(|| format!("write {}", cache_path.display()))?;
        serde_json::from_str(&contents).with_context(|| format!("parse {}", source_path.display()))
    }
}

fn read_json(path: &Path) -> Result<Value> {
    let contents = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&contents).with_context(|| format!("parse {}", path.display()))
}
