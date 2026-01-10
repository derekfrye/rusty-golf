#![cfg(target_arch = "wasm32")]

use chrono::Utc;
use rusty_golf_core::storage::StorageError;
use std::collections::HashMap;

use crate::storage_helpers::parse_rfc3339;
use crate::storage_types::{AdminEupDataFill, AdminSeedRequest, GolferAssignment, PlayerFactorEntry};

pub fn validate_seed_request(
    request: &AdminSeedRequest,
) -> Result<&AdminEupDataFill, StorageError> {
    if request.event_id != request.event.event as i32 {
        return Err(StorageError::new(format!(
            "event_id mismatch: request {}, event {}",
            request.event_id, request.event.event
        )));
    }

    request
        .event
        .data_to_fill_if_event_and_year_missing
        .first()
        .ok_or_else(|| StorageError::new("missing data_to_fill_if_event_and_year_missing"))
}

pub fn build_golfers_out(
    event_id: i32,
    data_to_fill: &AdminEupDataFill,
) -> Result<Vec<GolferAssignment>, StorageError> {
    let mut golfers_by_id = HashMap::new();
    for golfer in &data_to_fill.golfers {
        golfers_by_id.insert(golfer.espn_id, golfer.name.as_str());
    }

    let mut bettor_counts: HashMap<&str, usize> = HashMap::new();
    let mut golfers_out = Vec::new();
    let mut eup_id = 1_i64;
    for entry in &data_to_fill.event_user_player {
        let count = bettor_counts.entry(entry.bettor.as_str()).or_insert(0);
        *count += 1;

        let golfer_name = golfers_by_id
            .get(&entry.golfer_espn_id)
            .ok_or_else(|| {
                StorageError::new(format!(
                    "missing golfer_espn_id {} for event {}",
                    entry.golfer_espn_id, event_id
                ))
            })?;

        golfers_out.push(GolferAssignment {
            eup_id,
            espn_id: entry.golfer_espn_id,
            golfer_name: (*golfer_name).to_string(),
            bettor_name: entry.bettor.clone(),
            group: *count as i64,
            score_view_step_factor: entry
                .score_view_step_factor
                .as_ref()
                .and_then(|value| value.as_f64().map(|num| num as f32)),
        });
        eup_id += 1;
    }

    Ok(golfers_out)
}

pub fn build_player_factors(data_to_fill: &AdminEupDataFill) -> Vec<PlayerFactorEntry> {
    data_to_fill
        .event_user_player
        .iter()
        .filter_map(|entry| {
            entry.score_view_step_factor.as_ref().and_then(|factor| {
                factor.as_f64().map(|num| PlayerFactorEntry {
                    golfer_espn_id: entry.golfer_espn_id,
                    bettor_name: entry.bettor.clone(),
                    step_factor: num as f32,
                })
            })
        })
        .collect()
}

pub fn resolve_last_refresh_ts(
    request: &AdminSeedRequest,
) -> Result<chrono::NaiveDateTime, StorageError> {
    if let Some(ts) = request.last_refresh.as_ref() {
        parse_rfc3339(ts).map_err(|e| StorageError::new(e.to_string()))
    } else {
        Ok(Utc::now().naive_utc())
    }
}
