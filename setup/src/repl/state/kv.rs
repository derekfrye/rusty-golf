use super::ReplState;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

use super::kv_helpers::{kv_key_get, kv_list_keys, parse_event_id_from_key};

#[derive(Deserialize)]
struct KvGolferAssignment {
    bettor_name: String,
    golfer_name: String,
    espn_id: i64,
}

#[derive(Deserialize)]
struct KvEventDetails {
    event_name: String,
}

pub(crate) fn load_current_golfers_by_bettor(
    state: &ReplState,
    event_id: i64,
) -> Result<Option<BTreeMap<String, Vec<String>>>> {
    let Some(assignments) = load_kv_golfers(state, event_id)? else {
        return Ok(None);
    };
    Ok(Some(build_golfers_by_bettor(&assignments)))
}

pub(crate) fn load_kv_bettors(state: &ReplState, event_id: i64) -> Result<Option<Vec<String>>> {
    let Some(assignments) = load_kv_golfers(state, event_id)? else {
        return Ok(None);
    };
    let mut bettors = BTreeSet::new();
    for entry in assignments {
        bettors.insert(entry.bettor_name);
    }
    Ok(Some(bettors.into_iter().collect()))
}

pub(crate) fn load_kv_golfers_list(
    state: &ReplState,
    event_id: i64,
) -> Result<Option<Vec<(String, i64)>>> {
    let Some(assignments) = load_kv_golfers(state, event_id)? else {
        return Ok(None);
    };
    let mut golfers = BTreeMap::new();
    for entry in assignments {
        golfers.entry(entry.golfer_name).or_insert(entry.espn_id);
    }
    Ok(Some(golfers.into_iter().collect()))
}

pub(crate) fn list_kv_event_ids(state: &ReplState) -> Result<Option<Vec<i64>>> {
    let Some(raw_keys) = kv_list_keys(state, "event:")? else {
        return Ok(None);
    };
    let mut event_ids = BTreeSet::new();
    for key in raw_keys {
        if let Some(event_id) = parse_event_id_from_key(&key) {
            event_ids.insert(event_id);
        }
    }
    Ok(Some(event_ids.into_iter().collect()))
}

pub(crate) fn load_kv_event_name(state: &ReplState, event_id: i64) -> Result<Option<String>> {
    let key = format!("event:{event_id}:details");
    let Some(raw) = kv_key_get(state, &key)? else {
        return Ok(None);
    };
    let details: KvEventDetails =
        serde_json::from_str(&raw).context("parse kv event details json")?;
    Ok(Some(details.event_name))
}

fn load_kv_golfers(state: &ReplState, event_id: i64) -> Result<Option<Vec<KvGolferAssignment>>> {
    let key = format!("event:{event_id}:golfers");
    let Some(raw) = kv_key_get(state, &key)? else {
        return Ok(None);
    };
    let assignments: Vec<KvGolferAssignment> =
        serde_json::from_str(&raw).context("parse kv golfers json")?;
    Ok(Some(assignments))
}

fn build_golfers_by_bettor(assignments: &[KvGolferAssignment]) -> BTreeMap<String, Vec<String>> {
    let mut golfers_by_bettor: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in assignments {
        golfers_by_bettor
            .entry(entry.bettor_name.clone())
            .or_default()
            .push(format!("{} ({})", entry.golfer_name, entry.espn_id));
    }
    golfers_by_bettor
}
