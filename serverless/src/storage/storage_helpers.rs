#![cfg(target_arch = "wasm32")]

use chrono::{DateTime, NaiveDateTime, Utc};

pub fn parse_rfc3339(value: &str) -> Result<NaiveDateTime, chrono::ParseError> {
    Ok(DateTime::parse_from_rfc3339(value)?.naive_utc())
}

pub fn format_rfc3339(value: NaiveDateTime) -> String {
    DateTime::<Utc>::from_naive_utc_and_offset(value, Utc).to_rfc3339()
}

pub fn parse_event_id(key: &str, suffix: &str) -> Option<i32> {
    let prefix = "event:";
    if !key.starts_with(prefix) || !key.ends_with(suffix) {
        return None;
    }
    let start = prefix.len();
    let end = key.len().saturating_sub(suffix.len());
    key.get(start..end)?.parse().ok()
}
