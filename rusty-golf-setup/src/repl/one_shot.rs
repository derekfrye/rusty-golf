use crate::config::GolferByBettorInput;
use crate::espn::EspnClient;
use crate::repl::payload::write_event_payload;
use crate::repl::state::{
    GolferSelection, ReplState, ensure_list_events, eup_event_exists, load_event_golfers,
};
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Run the one-shot event setup flow.
///
/// # Errors
/// Returns an error if the event data cannot be loaded or written.
pub fn run_new_event_one_shot(
    eup_json: Option<PathBuf>,
    output_json: &Path,
    event_id: i64,
    golfers_by_bettor: Vec<GolferByBettorInput>,
) -> Result<()> {
    run_new_event_one_shot_with_client(
        eup_json,
        output_json,
        event_id,
        golfers_by_bettor,
        None,
    )
}

/// Run the one-shot event setup flow with an injected ESPN client.
///
/// # Errors
/// Returns an error if the event data cannot be loaded or written.
pub fn run_new_event_one_shot_with_client(
    eup_json: Option<PathBuf>,
    output_json: &Path,
    event_id: i64,
    golfers_by_bettor: Vec<GolferByBettorInput>,
    espn: Option<Arc<dyn EspnClient>>,
) -> Result<()> {
    let output_json_path = output_json.to_path_buf();
    let mut state = match espn {
        Some(client) => ReplState::new_with_client(eup_json, Some(output_json_path), client)
            .context("init repl state")?,
        None => ReplState::new(eup_json, Some(output_json_path)).context("init repl state")?,
    };
    let events = ensure_list_events(&mut state, false, true)?;
    if events.is_empty() {
        return Err(anyhow::anyhow!("no events found"));
    }

    let event_id_raw = event_id.to_string();
    let event_name = events
        .iter()
        .find(|(id, _)| *id == event_id_raw)
        .map_or_else(|| event_id_raw.clone(), |(_, name)| name.clone());

    if eup_event_exists(&state, event_id)? {
        println!("Warning: event {event_id} already exists in eup json.");
    }

    let golfers = load_event_golfers(&state, &event_id_raw)?;
    if golfers.is_empty() {
        return Err(anyhow::anyhow!("no golfers found for event {event_id}"));
    }

    let mut bettors = Vec::new();
    let mut seen_bettors = BTreeSet::new();
    let mut selections = Vec::new();
    for entry in golfers_by_bettor {
        if seen_bettors.insert(entry.bettor.clone()) {
            bettors.push(entry.bettor.clone());
        }
        let golfer_id = match_golfer_exact(&golfers, &entry.golfer)?;
        selections.push(GolferSelection {
            bettor: entry.bettor,
            golfer_espn_id: golfer_id,
        });
    }

    write_event_payload(
        &state,
        output_json,
        event_id,
        &event_name,
        &golfers,
        &bettors,
        &selections,
    )
}

fn match_golfer_exact(golfers: &[(String, i64)], golfer: &str) -> Result<i64> {
    if let Some((_, id)) = golfers.iter().find(|(name, _)| name == golfer) {
        return Ok(*id);
    }
    let suggestions = closest_golfers(golfers, golfer);
    if suggestions.is_empty() {
        return Err(anyhow::anyhow!("Unknown golfer: {golfer}"));
    }
    Err(anyhow::anyhow!(
        "Unknown golfer: {golfer}. Closest matches: {}",
        suggestions.join(", ")
    ))
}

fn closest_golfers(golfers: &[(String, i64)], golfer: &str) -> Vec<String> {
    let target = golfer.to_lowercase();
    let mut ranked: Vec<(f64, &String)> = golfers
        .iter()
        .map(|(name, _)| (strsim::jaro_winkler(&name.to_lowercase(), &target), name))
        .collect();
    ranked.sort_by(|(score_a, name_a), (score_b, name_b)| {
        score_b
            .partial_cmp(score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| name_a.cmp(name_b))
    });
    ranked
        .into_iter()
        .filter(|(score, _)| *score > 0.0)
        .take(3)
        .map(|(_, name)| name.clone())
        .collect()
}
