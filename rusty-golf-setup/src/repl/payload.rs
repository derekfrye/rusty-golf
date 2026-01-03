use crate::repl::state::{GolferSelection, ReplState, load_eup_json};
use anyhow::{Context, Result};
use chrono::Datelike;
use serde_json::Value;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub fn write_event_payload(
    state: &ReplState,
    output_path: &Path,
    event_id: i64,
    event_name: &str,
    golfers: &[(String, i64)],
    bettors: &[String],
    selections: &[GolferSelection],
) -> Result<()> {
    let existing = load_eup_json(state)?;
    let year = chrono::Utc::now().year();
    let event_user_player = build_event_user_player(selections);
    let golfers_payload = build_golfers_payload(golfers, selections)?;
    let new_event = build_new_event_json(
        event_id,
        year,
        event_name,
        bettors,
        &golfers_payload,
        &event_user_player,
    );

    let mut payload = existing;
    payload.push(new_event);
    let serialized = serde_json::to_string_pretty(&payload)?;
    std::fs::write(output_path, serialized)
        .with_context(|| format!("write {}", output_path.display()))?;
    println!("Wrote {}", output_path.display());
    Ok(())
}

fn build_event_user_player(selections: &[GolferSelection]) -> Vec<Value> {
    selections
        .iter()
        .map(|entry| {
            json!({
                "bettor": entry.bettor,
                "golfer_espn_id": entry.golfer_espn_id,
            })
        })
        .collect()
}

fn build_golfers_payload(
    golfers: &[(String, i64)],
    selections: &[GolferSelection],
) -> Result<Vec<Value>> {
    let golfers_by_id: HashMap<i64, &String> = golfers
        .iter()
        .map(|(name, id)| (*id, name))
        .collect();
    let mut seen_golfers = HashSet::new();
    let mut selected_golfers = Vec::new();
    for entry in selections {
        if !seen_golfers.insert(entry.golfer_espn_id) {
            continue;
        }
        let name = golfers_by_id
            .get(&entry.golfer_espn_id)
            .ok_or_else(|| anyhow::anyhow!("missing golfer {}", entry.golfer_espn_id))?;
        selected_golfers.push(((*name).clone(), entry.golfer_espn_id));
    }
    Ok(selected_golfers
        .iter()
        .map(|(name, id)| {
            json!({
                "name": name,
                "espn_id": id,
            })
        })
        .collect())
}

fn build_new_event_json(
    event_id: i64,
    year: i32,
    event_name: &str,
    bettors: &[String],
    golfers: &[Value],
    event_user_player: &[Value],
) -> Value {
    json!({
        "event": event_id,
        "year": year,
        "name": event_name,
        "score_view_step_factor": 3.0,
        "data_to_fill_if_event_and_year_missing": [
            {
                "bettors": bettors,
                "golfers": golfers,
                "event_user_player": event_user_player,
            }
        ],
    })
}
