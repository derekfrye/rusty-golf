use crate::espn::EspnClient;
use anyhow::{Context, Result};
use chrono::{DateTime, Local, Timelike, TimeZone};
use chrono_tz::Tz;
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
    let mut start_date = extract_event_field(
        &payload,
        &[
            &["event", "startDate"],
            &["event", "date"],
            &["startDate"],
            &["date"],
        ],
    );
    let mut end_date = extract_event_field(&payload, &[&["event", "endDate"], &["endDate"]]);
    if start_date.is_none() || end_date.is_none() {
        if let Ok(scoreboard) = espn.fetch_scoreboard_header_cached(cache_dir) {
            if let Some((header_start, header_end)) =
                extract_dates_from_scoreboard(&scoreboard, event_id)
            {
                if start_date.is_none() {
                    start_date = header_start;
                }
                if end_date.is_none() {
                    end_date = header_end;
                }
            }
        }
    }
    let start_date = start_date
        .map(|value| format_event_date_local(&value).unwrap_or(value))
        .unwrap_or_else(|| "-".to_string());
    let end_date = end_date
        .map(|value| format_event_date_local(&value).unwrap_or(value))
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

fn format_event_date_local(value: &str) -> Option<String> {
    let parsed = DateTime::parse_from_rfc3339(value).ok()?;
    if let Some(tz) = resolve_tz() {
        let local = parsed.with_timezone(&tz);
        return Some(format_event_date(local));
    }
    let local = parsed.with_timezone(&Local);
    Some(format_event_date(local))
}

fn resolve_tz() -> Option<Tz> {
    std::env::var("TZ")
        .ok()
        .and_then(|value| value.parse::<Tz>().ok())
}

fn format_event_date<TzType>(local: DateTime<TzType>) -> String
where
    TzType: TimeZone,
    TzType::Offset: std::fmt::Display,
{
    let date = local.format("%b %-d %Y").to_string();
    let (is_pm, hour12) = local.time().hour12();
    let suffix = if is_pm { "p" } else { "a" };
    let tz_abbr = local.format("%Z").to_string();
    format!(
        "{}, {}:{:02}{} {}",
        date,
        hour12,
        local.minute(),
        suffix,
        tz_abbr
    )
}

fn extract_dates_from_scoreboard(
    payload: &Value,
    event_id: i64,
) -> Option<(Option<String>, Option<String>)> {
    let sports = payload.get("sports").and_then(Value::as_array)?;
    let event_id_str = event_id.to_string();
    for sport in sports {
        let leagues = sport.get("leagues").and_then(Value::as_array);
        for league in leagues.into_iter().flatten() {
            let events = league.get("events").and_then(Value::as_array);
            for event in events.into_iter().flatten() {
                let id = event.get("id").and_then(Value::as_str);
                if id != Some(event_id_str.as_str()) {
                    continue;
                }
                let start_date = event.get("date").and_then(Value::as_str).map(str::to_string);
                let end_date = event.get("endDate").and_then(Value::as_str).map(str::to_string);
                return Some((start_date, end_date));
            }
        }
    }
    None
}
