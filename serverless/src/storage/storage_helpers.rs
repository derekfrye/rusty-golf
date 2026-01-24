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

pub fn parse_year_from_end_date(end_date: Option<&str>) -> Option<i32> {
    let value = end_date?.trim();
    if value.len() < 4 {
        return None;
    }
    let year_bytes = value.as_bytes();
    if !year_bytes[..4].iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value[..4].parse().ok()
}
