use crate::espn::MalformedEspnJson;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::BTreeMap;
use std::time::Duration;

use super::cache::warm_event_cache;
use super::eup::read_eup_event_ids;
use super::{ReplState, list_kv_event_ids, load_kv_event_name};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EventListMode {
    EnsureAll,
    RefreshEspn,
    RefreshKv,
    RefreshAll,
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
    mode: EventListMode,
    warm_cache: bool,
) -> Result<Vec<(String, String)>> {
    if state.cached_events.is_some() && matches!(mode, EventListMode::EnsureAll) {
        return Ok(state.cached_events.clone().unwrap_or_default());
    }

    let espn_events = match mode {
        EventListMode::EnsureAll => {
            if let Some(events) = state.cached_espn_events.clone() {
                events
            } else {
                let events = load_espn_events(state)?;
                state.cached_espn_events = Some(events.clone());
                events
            }
        }
        EventListMode::RefreshEspn | EventListMode::RefreshAll => {
            let events = load_espn_events(state)?;
            state.cached_espn_events = Some(events.clone());
            events
        }
        EventListMode::RefreshKv => state.cached_espn_events.clone().unwrap_or_default(),
    };

    let kv_events = match mode {
        EventListMode::EnsureAll => {
            if let Some(events) = state.cached_kv_events.clone() {
                events
            } else {
                let events = load_kv_events(state);
                state.cached_kv_events = Some(events.clone());
                events
            }
        }
        EventListMode::RefreshKv | EventListMode::RefreshAll => {
            let events = load_kv_events(state);
            state.cached_kv_events = Some(events.clone());
            events
        }
        EventListMode::RefreshEspn => state.cached_kv_events.clone().unwrap_or_default(),
    };

    let cached = merge_events(espn_events, kv_events);
    if warm_cache {
        warm_event_cache(&cached, &state.event_cache_dir, &state.espn)?;
    }
    state.cached_events = Some(cached.clone());
    Ok(cached)
}

fn merge_events(
    espn_events: Vec<(String, String)>,
    kv_events: Vec<(String, String)>,
) -> Vec<(String, String)> {
    let mut events = BTreeMap::new();
    for (id, name) in espn_events {
        events.insert(id, name);
    }
    for (id, name) in kv_events {
        if name.is_empty() {
            events.entry(id).or_insert_with(String::new);
        } else {
            events.insert(id, name);
        }
    }
    events.into_iter().collect()
}

fn load_espn_events(state: &ReplState) -> Result<Vec<(String, String)>> {
    let overall_style =
        ProgressStyle::with_template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar());

    let mut events = BTreeMap::new();
    let mut missing_ids: Vec<i64> = Vec::new();
    let overall = ProgressBar::new(1);
    overall.set_style(overall_style);
    overall.set_message("Fetching events");
    overall.enable_steady_tick(Duration::from_millis(120));

    let fetched_events = match state.espn.list_events() {
        Ok(events) => {
            overall.inc(1);
            events
        }
        Err(err) => {
            overall.abandon_with_message("Fetching events failed");
            return Err(err);
        }
    };
    for (id, name) in fetched_events {
        events.insert(id, name);
    }

    if let Some(path) = state.eup_json_path.as_ref() {
        let eup_event_ids = read_eup_event_ids(path)?;
        missing_ids = eup_event_ids
            .into_iter()
            .filter(|event_id| {
                let key = event_id.to_string();
                match events.get(&key) {
                    Some(name) => name.is_empty(),
                    None => true,
                }
            })
            .collect();
    }
    overall.set_length(1 + missing_ids.len() as u64);

    if !missing_ids.is_empty() {
        overall.set_message("Fetching missing event names");
        for (event_id, name) in state.espn.fetch_event_names_parallel(
            &missing_ids,
            &state.event_cache_dir,
            Some(&overall),
        ) {
            events.insert(event_id.to_string(), name);
        }
    }
    overall.finish_and_clear();

    Ok(events.into_iter().collect())
}

fn load_kv_events(state: &ReplState) -> Vec<(String, String)> {
    let overall_style =
        ProgressStyle::with_template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar());
    let overall = ProgressBar::new(0);
    overall.set_style(overall_style);
    overall.set_message("Loading KV events");
    overall.enable_steady_tick(Duration::from_millis(120));

    let kv_event_ids = match list_kv_event_ids(state) {
        Ok(Some(ids)) => ids,
        Ok(None) => {
            overall.finish_and_clear();
            return Vec::new();
        }
        Err(err) => {
            println!("Warning: failed to list KV events: {err}");
            overall.finish_and_clear();
            return Vec::new();
        }
    };

    overall.set_length(kv_event_ids.len() as u64);
    let mut events = BTreeMap::new();
    for event_id in kv_event_ids {
        let id = event_id.to_string();
        let name = match load_kv_event_name(state, event_id) {
            Ok(name) => name,
            Err(err) => {
                println!("Warning: failed to load KV event {event_id}: {err}");
                None
            }
        };
        if let Some(name) = name {
            events.insert(id, name);
        } else {
            events.entry(id).or_insert_with(String::new);
        }
        overall.inc(1);
    }
    overall.finish_and_clear();
    events.into_iter().collect()
}
