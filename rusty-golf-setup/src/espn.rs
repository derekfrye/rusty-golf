use anyhow::{Context, Result};
use rayon::prelude::*;
use serde_json::Value;
use std::fmt;
use std::fs;
use std::path::Path;

pub const ESPN_SCOREBOARD_URL: &str = "https://site.web.api.espn.com/apis/v2/scoreboard/header?sport=golf&league=pga&region=us&lang=en&contentorigin=espn";
pub const ESPN_EVENT_URL_PREFIX: &str =
    "https://site.web.api.espn.com/apis/site/v2/sports/golf/pga/leaderboard/players?region=us&lang=en&event=";

#[derive(Debug)]
pub struct MalformedEspnJson;

impl fmt::Display for MalformedEspnJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "malformed ESPN JSON")
    }
}

impl std::error::Error for MalformedEspnJson {}

pub fn list_espn_events() -> Result<Vec<(String, String)>> {
    let response = reqwest::blocking::get(ESPN_SCOREBOARD_URL)
        .context("fetch ESPN events")?
        .text()
        .context("read ESPN response body")?;
    let payload: Value = serde_json::from_str(&response)
        .map_err(|_| anyhow::Error::new(MalformedEspnJson))?;
    Ok(extract_espn_events(&payload))
}

pub fn fetch_event_name(event_id: i64, cache_dir: &Path) -> Result<String> {
    let payload = fetch_event_json_cached(event_id, cache_dir)?;
    let name = payload
        .get("event")
        .and_then(|event| event.get("name"))
        .and_then(Value::as_str)
        .or_else(|| payload.get("name").and_then(Value::as_str))
        .ok_or_else(|| anyhow::Error::new(MalformedEspnJson))?;
    Ok(name.to_string())
}

pub fn fetch_event_names_parallel(event_ids: &[i64], cache_dir: &Path) -> Vec<(i64, String)> {
    if event_ids.is_empty() {
        return Vec::new();
    }
    let pool = match rayon::ThreadPoolBuilder::new().num_threads(4).build() {
        Ok(pool) => pool,
        Err(_) => {
            return event_ids
                .iter()
                .filter_map(|event_id| {
                    fetch_event_name(*event_id, cache_dir)
                        .ok()
                        .map(|name| (*event_id, name))
                })
                .collect();
        }
    };
    pool.install(|| {
        event_ids
            .par_iter()
            .filter_map(|event_id| {
                fetch_event_name(*event_id, cache_dir)
                    .ok()
                    .map(|name| (*event_id, name))
            })
            .collect()
    })
}

pub fn fetch_event_json_cached(event_id: i64, cache_dir: &Path) -> Result<Value> {
    let cache_path = cache_dir.join(format!("{event_id}.json"));
    if cache_path.is_file() {
        let contents = fs::read_to_string(&cache_path)
            .with_context(|| format!("read {}", cache_path.display()))?;
        let payload: Value =
            serde_json::from_str(&contents).map_err(|_| anyhow::Error::new(MalformedEspnJson))?;
        return Ok(payload);
    }

    let url = format!("{ESPN_EVENT_URL_PREFIX}{event_id}");
    let response = reqwest::blocking::get(&url)
        .context("fetch ESPN event")?
        .text()
        .context("read ESPN event response body")?;
    let payload: Value = serde_json::from_str(&response)
        .map_err(|_| anyhow::Error::new(MalformedEspnJson))?;
    fs::write(&cache_path, response)
        .with_context(|| format!("write {}", cache_path.display()))?;
    Ok(payload)
}

pub fn extract_espn_events(payload: &Value) -> Vec<(String, String)> {
    let mut events = Vec::new();
    let sports = payload.get("sports").and_then(Value::as_array);
    for sport in sports.into_iter().flatten() {
        let leagues = sport.get("leagues").and_then(Value::as_array);
        for league in leagues.into_iter().flatten() {
            let entries = league.get("events").and_then(Value::as_array);
            for event in entries.into_iter().flatten() {
                let id = event.get("id").and_then(Value::as_str);
                let name = event
                    .get("name")
                    .and_then(Value::as_str)
                    .or_else(|| event.get("shortName").and_then(Value::as_str));
                if let (Some(id), Some(name)) = (id, name) {
                    events.push((id.to_string(), name.to_string()));
                }
            }
        }
    }
    events
}
