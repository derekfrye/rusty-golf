use crate::error::CoreError;
use crate::storage::Storage;
use chrono::Utc;
use std::collections::HashMap;
use std::hash::BuildHasher;

#[derive(Debug, Clone, Copy)]
pub struct ScoreRequest {
    pub event_id: i32,
    pub year: i32,
    pub use_cache: bool,
    pub want_json: bool,
    pub expanded: bool,
}

/// Parse query parameters into a score request.
///
/// # Errors
/// Returns an error if required query parameters are missing or invalid.
pub fn parse_score_request<S: BuildHasher>(
    query: &HashMap<String, String, S>,
) -> Result<ScoreRequest, CoreError> {
    let event_id = query
        .get("event")
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| CoreError::Other("espn event parameter is required".into()))?;
    let year = query
        .get("yr")
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| CoreError::Other("yr (year) parameter is required".into()))?;
    let use_cache = !matches!(query.get("cache").map(String::as_str), Some("0"));
    let want_json = match query.get("json").map(String::as_str) {
        Some("1") => true,
        Some("0") | None => false,
        Some(other) => other.parse().unwrap_or(false),
    };
    let expanded = match query.get("expanded").map(String::as_str) {
        Some("1") => true,
        Some("0") | None => false,
        Some(other) => other.parse().unwrap_or(false),
    };
    Ok(ScoreRequest {
        event_id,
        year,
        use_cache,
        want_json,
        expanded,
    })
}

/// Fetch cache max age for an event.
///
/// # Errors
/// Returns an error if event details cannot be retrieved.
pub async fn cache_max_age_for_event(
    storage: &dyn Storage,
    event_id: i32,
) -> Result<i64, CoreError> {
    let event_details = match storage.get_event_details(event_id).await {
        Ok(details) => details,
        Err(_) => return Ok(0),
    };

    if let Some(end_date) = event_details.end_date.as_deref() {
        if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(end_date) {
            let end_utc = parsed.with_timezone(&Utc);
            if Utc::now() > end_utc {
                return Ok(-1);
            }
        }
    }

    Ok(match event_details.refresh_from_espn {
        1 => 300,
        _ => 0,
    })
}

/// Parse a score request and build a derived value.
///
/// # Errors
/// Returns an error if parsing the request or loading cache settings fails.
pub async fn decode_score_request<T, S: BuildHasher>(
    query: &HashMap<String, String, S>,
    storage: &dyn Storage,
    builder: impl FnOnce(ScoreRequest, i64) -> T,
) -> Result<T, CoreError> {
    let score_request = parse_score_request(query)?;
    let cache_max_age = cache_max_age_for_event(storage, score_request.event_id).await?;
    Ok(builder(score_request, cache_max_age))
}
