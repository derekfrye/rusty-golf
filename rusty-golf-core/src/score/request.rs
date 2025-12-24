use crate::error::CoreError;
use crate::storage::Storage;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct ScoreRequest {
    pub event_id: i32,
    pub year: i32,
    pub use_cache: bool,
    pub want_json: bool,
    pub expanded: bool,
}

#[allow(clippy::implicit_hasher)]
pub fn parse_score_request(query: &HashMap<String, String>) -> Result<ScoreRequest, CoreError> {
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

pub async fn cache_max_age_for_event(
    storage: &dyn Storage,
    event_id: i32,
) -> Result<i64, CoreError> {
    let cache_max_age = match storage.get_event_details(event_id).await {
        Ok(event_details) => match event_details.refresh_from_espn {
            1 => 99,
            _ => 0,
        },
        Err(_) => 0,
    };
    Ok(cache_max_age)
}

pub async fn decode_score_request<T>(
    query: &HashMap<String, String>,
    storage: &dyn Storage,
    builder: impl FnOnce(ScoreRequest, i64) -> T,
) -> Result<T, CoreError> {
    let score_request = parse_score_request(query)?;
    let cache_max_age = cache_max_age_for_event(storage, score_request.event_id).await?;
    Ok(builder(score_request, cache_max_age))
}
