use crate::espn::EspnClient;
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use tabled::Tabled;

#[derive(Debug, Serialize, Tabled)]
pub struct EventDetailsRow {
    pub event_id: i64,
    pub event_name: String,
    pub start_date: String,
    pub end_date: String,
}

/// Build a single event details row from the ESPN event payload.
///
/// # Errors
/// Returns an error if the event payload cannot be fetched or parsed.
pub fn build_event_details_row(
    event_id: i64,
    event_name_hint: Option<&str>,
    espn: &dyn EspnClient,
    cache_dir: &Path,
) -> Result<EventDetailsRow> {
    let payload = espn
        .fetch_event_json_cached(event_id, cache_dir)
        .with_context(|| format!("load event {event_id}"))?;
    let event_name = event_name_hint
        .map(str::to_string)
        .or_else(|| extract_event_name(&payload))
        .unwrap_or_else(|| "Unknown".to_string());
    let start_date = extract_event_field(
        &payload,
        &[
            &["event", "startDate"],
            &["event", "date"],
            &["startDate"],
            &["date"],
        ],
    )
    .unwrap_or_else(|| "-".to_string());
    let end_date = extract_event_field(&payload, &[&["event", "endDate"], &["endDate"]])
        .unwrap_or_else(|| "-".to_string());
    Ok(EventDetailsRow {
        event_id,
        event_name,
        start_date,
        end_date,
    })
}

fn extract_event_name(payload: &Value) -> Option<String> {
    let paths = [
        &["event", "name"][..],
        &["event", "shortName"][..],
        &["name"][..],
        &["shortName"][..],
    ];
    extract_event_field(payload, &paths)
}

fn extract_event_field(payload: &Value, paths: &[&[&str]]) -> Option<String> {
    for path in paths {
        let mut current = payload;
        let mut missing = false;
        for key in *path {
            if let Some(next) = current.get(*key) {
                current = next;
            } else {
                missing = true;
                break;
            }
        }
        if missing {
            continue;
        }
        if let Some(value) = current.as_str()
            && !value.trim().is_empty()
        {
            return Some(value.to_string());
        }
    }
    None
}
