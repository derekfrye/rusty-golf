use crate::config::GolferByBettorInput;
use crate::event_details::{EventDetailsRow, build_event_details_row};
use crate::espn::EspnClient;
use crate::repl::payload::{build_event_payload_string, write_event_payload};
use crate::repl::state::{
    GolferSelection, ReplState, ensure_list_events, eup_event_exists, load_event_golfers,
};
use anyhow::{Context, Result, anyhow};
use serde_json::to_string_pretty;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Run the one-shot event setup flow.
///
/// # Errors
/// Returns an error if the event data cannot be loaded or written.
pub fn run_new_event_one_shot(
    eup_json: Option<PathBuf>,
    output_json: Option<&Path>,
    output_json_stdout: bool,
    event_id: i64,
    golfers_by_bettor: Vec<GolferByBettorInput>,
) -> Result<()> {
    run_new_event_one_shot_with_client(
        eup_json,
        output_json,
        output_json_stdout,
        event_id,
        golfers_by_bettor,
        None,
    )
}

/// Run the one-shot event details flow.
///
/// # Errors
/// Returns an error if event details cannot be fetched or written.
pub fn run_get_event_details_one_shot(
    output_json: Option<&Path>,
    output_json_stdout: bool,
    event_ids: Option<Vec<i64>>,
) -> Result<()> {
    run_get_event_details_one_shot_with_client(output_json, output_json_stdout, event_ids, None)
}

/// Run the one-shot event details flow with an injected ESPN client.
///
/// # Errors
/// Returns an error if event details cannot be fetched or written.
pub fn run_get_event_details_one_shot_with_client(
    output_json: Option<&Path>,
    output_json_stdout: bool,
    event_ids: Option<Vec<i64>>,
    espn: Option<Arc<dyn EspnClient>>,
) -> Result<()> {
    let mut state = match espn {
        Some(client) => ReplState::new_with_client(None, None, client).context("init repl state")?,
        None => ReplState::new(None, None).context("init repl state")?,
    };
    let (event_ids, event_names) = if let Some(ids) = event_ids {
        (ids, std::collections::BTreeMap::new())
    } else {
        let events = ensure_list_events(&mut state, false, true)?;
        if events.is_empty() {
            return Err(anyhow!("no events found"));
        }
        let mut ids = Vec::new();
        let mut names = std::collections::BTreeMap::new();
        for (id, name) in events {
            if let Ok(parsed) = id.parse::<i64>() {
                ids.push(parsed);
                names.insert(id, name);
            }
        }
        (ids, names)
    };
    if event_ids.is_empty() {
        return Err(anyhow!("no events found"));
    }

    let mut rows = Vec::new();
    for event_id in event_ids {
        let event_name_hint = event_names.get(&event_id.to_string()).map(String::as_str);
        match build_event_details_row(
            event_id,
            event_name_hint,
            state.espn.as_ref(),
            &state.event_cache_dir,
        ) {
            Ok(row) => rows.push(row),
            Err(err) => eprintln!("Failed to load event {event_id}: {err}"),
        }
    }

    write_event_details(output_json, output_json_stdout, &rows)
}

/// Run the one-shot event setup flow with an injected ESPN client.
///
/// # Errors
/// Returns an error if the event data cannot be loaded or written.
pub fn run_new_event_one_shot_with_client(
    eup_json: Option<PathBuf>,
    output_json: Option<&Path>,
    output_json_stdout: bool,
    event_id: i64,
    golfers_by_bettor: Vec<GolferByBettorInput>,
    espn: Option<Arc<dyn EspnClient>>,
) -> Result<()> {
    let output_json_path = output_json.map(Path::to_path_buf);
    let mut state = match espn {
        Some(client) => ReplState::new_with_client(eup_json, output_json_path, client)
            .context("init repl state")?,
        None => ReplState::new(eup_json, output_json_path).context("init repl state")?,
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

    if output_json_stdout {
        let payload = build_event_payload_string(
            &state,
            event_id,
            &event_name,
            &golfers,
            &bettors,
            &selections,
        )?;
        println!("{payload}");
        return Ok(());
    }
    let Some(output_json) = output_json else {
        return Err(anyhow!("missing output path for new event payload"));
    };
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

fn write_event_details(
    path: Option<&Path>,
    output_json_stdout: bool,
    rows: &[EventDetailsRow],
) -> Result<()> {
    let payload = to_string_pretty(rows).context("serialize event details")?;
    if output_json_stdout {
        println!("{payload}");
        return Ok(());
    }
    let Some(path) = path else {
        return Err(anyhow!("missing output path for event details"));
    };
    fs::write(path, payload).with_context(|| format!("write {}", path.display()))?;
    Ok(())
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
