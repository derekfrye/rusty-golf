use crate::espn::MalformedEspnJson;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::BTreeMap;
use std::time::Duration;

use super::ReplState;
use super::cache::warm_event_cache;
use super::eup::read_eup_event_ids;

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
            .filter(|event_id| !events.contains_key(&event_id.to_string()))
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

    let cached: Vec<(String, String)> = events.into_iter().collect();
    if warm_cache {
        warm_event_cache(&cached, &state.event_cache_dir, &state.espn)?;
    }
    state.cached_events = Some(cached.clone());
    Ok(cached)
}
