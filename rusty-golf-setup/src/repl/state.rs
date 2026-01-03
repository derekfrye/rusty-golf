use crate::espn::{EspnClient, HttpEspnClient, MalformedEspnJson};
use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

pub(crate) struct ReplState {
    cached_events: Option<Vec<(String, String)>>,
    cached_bettors: Option<Vec<String>>,
    golfers_by_bettor: Option<Vec<GolferSelection>>,
    eup_json_path: Option<PathBuf>,
    event_cache_dir: PathBuf,
    bettors_selection_path: PathBuf,
    output_json_path: Option<PathBuf>,
    espn: Arc<dyn EspnClient>,
    _temp_dir: TempDir,
}

impl ReplState {
    pub(crate) fn new(
        eup_json_path: Option<PathBuf>,
        output_json_path: Option<PathBuf>,
    ) -> Result<Self> {
        Self::new_with_client(
            eup_json_path,
            output_json_path,
            Arc::new(HttpEspnClient),
        )
    }

    pub(crate) fn new_with_client(
        eup_json_path: Option<PathBuf>,
        output_json_path: Option<PathBuf>,
        espn: Arc<dyn EspnClient>,
    ) -> Result<Self> {
        let temp_dir = TempDir::new().context("create event cache dir")?;
        let event_cache_dir = temp_dir.path().join("espn_events");
        let bettors_selection_path = temp_dir.path().join("bettors.txt");
        fs::create_dir_all(&event_cache_dir)
            .with_context(|| format!("create {}", event_cache_dir.display()))?;
        Ok(Self {
            cached_events: None,
            cached_bettors: None,
            golfers_by_bettor: None,
            eup_json_path,
            event_cache_dir,
            bettors_selection_path,
            output_json_path,
            espn,
            _temp_dir: temp_dir,
        })
    }
}

#[derive(Clone)]
pub(crate) struct GolferSelection {
    pub(crate) bettor: String,
    pub(crate) golfer_espn_id: i64,
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
    warm_cache: bool,
) -> Result<Vec<(String, String)>> {
    if state.cached_events.is_some() && !refresh {
        return Ok(state.cached_events.clone().unwrap_or_default());
    }

    let multi = MultiProgress::new();
    let spinner = multi.add(ProgressBar::new_spinner());
    let spinner_style = ProgressStyle::with_template("{spinner} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
        .tick_chars("|/-\\");
    spinner.set_style(spinner_style);
    spinner.set_message("Fetching events");
    spinner.enable_steady_tick(Duration::from_millis(120));

    let mut events = BTreeMap::new();
    let fetched_events = match state.espn.list_events() {
        Ok(events) => {
            spinner.finish_and_clear();
            events
        }
        Err(err) => {
            spinner.abandon_with_message("Fetching events failed");
            return Err(err);
        }
    };
    for (id, name) in fetched_events {
        events.insert(id, name);
    }

    if let Some(path) = state.eup_json_path.as_ref() {
        let eup_event_ids = read_eup_event_ids(path)?;
        let missing_ids: Vec<i64> = eup_event_ids
            .into_iter()
            .filter(|event_id| !events.contains_key(&event_id.to_string()))
            .collect();
        if !missing_ids.is_empty() {
            let overall = multi.add(ProgressBar::new(missing_ids.len() as u64));
            let overall_style = ProgressStyle::with_template(
                "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}",
            )
            .unwrap_or_else(|_| ProgressStyle::default_bar());
            overall.set_style(overall_style);
            overall.set_message("Fetching missing event names");
            overall.enable_steady_tick(Duration::from_millis(120));
            for (event_id, name) in state.espn.fetch_event_names_parallel(
                &missing_ids,
                &state.event_cache_dir,
                Some(&overall),
            ) {
                events.insert(event_id.to_string(), name);
            }
            overall.finish_and_clear();
        }
    }

    let cached: Vec<(String, String)> = events.into_iter().collect();
    if warm_cache {
        warm_event_cache(&cached, &state.event_cache_dir, &state.espn)?;
    }
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

pub(crate) fn load_cached_golfers(
    state: &ReplState,
) -> Result<Vec<(String, i64)>> {
    let entries = fs::read_dir(&state.event_cache_dir)
        .with_context(|| format!("read {}", state.event_cache_dir.display()))?;
    let mut golfers = BTreeMap::new();
    for entry in entries {
        let path = entry
            .with_context(|| format!("read {}", state.event_cache_dir.display()))?
            .path();
        if path.extension().map_or(true, |ext| ext != "json") {
            continue;
        }
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("read {}", path.display()))?;
        let payload: Value =
            serde_json::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
        if let Some(leaderboard) = payload.get("leaderboard").and_then(Value::as_array) {
            for entry in leaderboard {
                let name = entry
                    .get("displayName")
                    .and_then(Value::as_str)
                    .or_else(|| entry.get("fullName").and_then(Value::as_str));
                let id = entry.get("id").and_then(Value::as_str);
                if let (Some(name), Some(id)) = (name, id) {
                    if let Ok(id) = id.parse::<i64>() {
                        golfers.entry(name.to_string()).or_insert(id);
                    }
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
    let payload: Value =
        serde_json::from_str(&contents).with_context(|| format!("parse {}", cache_path.display()))?;
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

pub(crate) fn set_golfers_by_bettor(
    state: &mut ReplState,
    selections: Vec<GolferSelection>,
) {
    state.golfers_by_bettor = Some(selections);
}

pub(crate) fn take_golfers_by_bettor(state: &mut ReplState) -> Option<Vec<GolferSelection>> {
    state.golfers_by_bettor.take()
}

pub(crate) fn output_json_path(state: &ReplState) -> Option<PathBuf> {
    state.output_json_path.clone()
}

fn warm_event_cache(
    events: &[(String, String)],
    cache_dir: &PathBuf,
    espn: &Arc<dyn EspnClient>,
) -> Result<()> {
    for (event_id, _) in events {
        let cache_path = cache_dir.join(format!("{event_id}.json"));
        if cache_path.is_file() {
            continue;
        }
        if let Ok(event_id) = event_id.parse::<i64>() {
            let _ = espn.fetch_event_name(event_id, cache_dir)?;
        }
    }
    Ok(())
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
