use crate::config::GolferByBettorInput;
use crate::espn::EspnClient;
use crate::event_details::build_event_details_row;
use crate::repl::payload::{build_event_payload_string, write_event_payload};
use crate::repl::state::{
    GolferSelection, ReplState, ensure_list_events, eup_event_exists, load_eup_event_dates,
    load_event_golfers,
};
use anyhow::{Context, Result, anyhow};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::one_shot_helpers::{match_golfer_exact, write_event_details};

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
    eup_json: Option<PathBuf>,
    output_json: Option<&Path>,
    output_json_stdout: bool,
    event_ids: Option<Vec<i64>>,
) -> Result<()> {
    run_get_event_details_one_shot_with_client(
        eup_json,
        output_json,
        output_json_stdout,
        event_ids,
        None,
    )
}

/// Run the one-shot event details flow with an injected ESPN client.
///
/// # Errors
/// Returns an error if event details cannot be fetched or written.
pub fn run_get_event_details_one_shot_with_client(
    eup_json: Option<PathBuf>,
    output_json: Option<&Path>,
    output_json_stdout: bool,
    event_ids: Option<Vec<i64>>,
    espn: Option<Arc<dyn EspnClient>>,
) -> Result<()> {
    let mut state = match espn {
        Some(client) => {
            ReplState::new_with_client(eup_json, None, None, client).context("init repl state")?
        }
        None => ReplState::new(eup_json, None, None).context("init repl state")?,
    };
    let (event_ids, event_names) = if let Some(ids) = event_ids {
        (ids, std::collections::BTreeMap::new())
    } else {
        let events = ensure_list_events(&mut state, crate::repl::state::EventListMode::EnsureAll, true)?;
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

    let eup_dates = match load_eup_event_dates(&state) {
        Ok(dates) => Some(dates),
        Err(err) => {
            eprintln!("Warning: failed to load eup dates: {err}");
            None
        }
    };
    let mut rows = Vec::new();
    for event_id in event_ids {
        let event_name_hint = event_names.get(&event_id.to_string()).map(String::as_str);
        let eup_dates = eup_dates.as_ref().and_then(|dates| dates.get(&event_id));
        match build_event_details_row(
            event_id,
            event_name_hint,
            state.espn.as_ref(),
            &state.event_cache_dir,
            eup_dates,
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
        Some(client) => ReplState::new_with_client(eup_json, output_json_path, None, client)
            .context("init repl state")?,
        None => ReplState::new(eup_json, output_json_path, None).context("init repl state")?,
    };
    let events = ensure_list_events(&mut state, crate::repl::state::EventListMode::EnsureAll, true)?;
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
