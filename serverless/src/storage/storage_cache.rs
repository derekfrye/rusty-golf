#![cfg(target_arch = "wasm32")]

use chrono::Utc;
use once_cell::sync::Lazy;
use rusty_golf_core::model::ScoresAndLastRefresh;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

pub const KV_SCORES_TTL_SECONDS: u64 = 300;
const IN_MEMORY_TTL_SECONDS: i64 = 30;
const PRE_EVENT_REFRESH_LEAD_SECONDS: i64 = 600;
const LONG_TTL_DAYS: i64 = 999;
const LONG_TTL_SECONDS: i64 = LONG_TTL_DAYS * 24 * 60 * 60;

#[derive(Serialize, Deserialize, Clone)]
pub struct KvScoresCacheEntry {
    pub cached_at: i64,
    #[serde(default = "default_kv_ttl_seconds")]
    pub ttl_seconds: i64,
    pub payload: ScoresAndLastRefresh,
}

#[derive(Serialize, Clone, Copy)]
pub struct CacheStatus {
    pub exists: bool,
    pub remaining_ttl_seconds: Option<i64>,
}

struct CacheEntry {
    value: ScoresAndLastRefresh,
    inserted_at: i64,
    ttl_seconds: i64,
}

static IN_MEMORY_SCORES: Lazy<Mutex<HashMap<i32, CacheEntry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn get_in_memory_scores(event_id: i32) -> Option<ScoresAndLastRefresh> {
    let now = Utc::now().timestamp();
    let mut cache = IN_MEMORY_SCORES.lock().ok()?;
    if let Some(entry) = cache.get(&event_id) {
        if now - entry.inserted_at <= entry.ttl_seconds {
            return Some(entry.value.clone());
        }
    }
    cache.remove(&event_id);
    None
}

pub fn in_memory_status(event_id: i32) -> CacheStatus {
    let now = Utc::now().timestamp();
    let Ok(mut cache) = IN_MEMORY_SCORES.lock() else {
        return CacheStatus {
            exists: false,
            remaining_ttl_seconds: None,
        };
    };
    let Some(entry) = cache.get(&event_id) else {
        return CacheStatus {
            exists: false,
            remaining_ttl_seconds: None,
        };
    };
    let age = now - entry.inserted_at;
    if age > entry.ttl_seconds {
        cache.remove(&event_id);
        return CacheStatus {
            exists: false,
            remaining_ttl_seconds: None,
        };
    }
    let remaining = (entry.ttl_seconds - age).max(0);
    CacheStatus {
        exists: true,
        remaining_ttl_seconds: Some(remaining),
    }
}

pub fn set_in_memory_scores(event_id: i32, scores: &ScoresAndLastRefresh, ttl_seconds: i64) {
    let now = Utc::now().timestamp();
    let Ok(mut cache) = IN_MEMORY_SCORES.lock() else {
        return;
    };
    cache.insert(
        event_id,
        CacheEntry {
            value: scores.clone(),
            inserted_at: now,
            ttl_seconds,
        },
    );
}

pub fn clear_in_memory_scores(event_id: i32) {
    let Ok(mut cache) = IN_MEMORY_SCORES.lock() else {
        return;
    };
    cache.remove(&event_id);
}

pub fn build_kv_scores_entry(
    scores: &ScoresAndLastRefresh,
    ttl_seconds: i64,
) -> KvScoresCacheEntry {
    KvScoresCacheEntry {
        cached_at: Utc::now().timestamp(),
        ttl_seconds,
        payload: scores.clone(),
    }
}

pub fn parse_kv_scores_entry(
    text: &str,
) -> Result<(ScoresAndLastRefresh, Option<i64>), serde_json::Error> {
    if let Ok(entry) = serde_json::from_str::<KvScoresCacheEntry>(text) {
        let remaining = kv_remaining_ttl_seconds(entry.cached_at, entry.ttl_seconds);
        return Ok((entry.payload, Some(remaining)));
    }
    let fallback = serde_json::from_str::<ScoresAndLastRefresh>(text)?;
    Ok((fallback, None))
}

pub fn kv_remaining_ttl_seconds(cached_at: i64, ttl_seconds: i64) -> i64 {
    let now = Utc::now().timestamp();
    let age = now - cached_at;
    let remaining = ttl_seconds - age;
    remaining.max(0)
}

pub fn derive_cache_ttls(
    start_date: Option<&str>,
    _end_date: Option<&str>,
    completed: bool,
) -> (u64, i64) {
    let now = Utc::now();
    if completed {
        return (LONG_TTL_SECONDS as u64, LONG_TTL_SECONDS);
    }

    if let Some(start_date) = start_date
        && let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(start_date)
    {
        let start_utc = parsed.with_timezone(&Utc);
        let cutoff = start_utc - chrono::Duration::seconds(PRE_EVENT_REFRESH_LEAD_SECONDS);
        if now < cutoff {
            let remaining = (cutoff - now).num_seconds().max(0);
            return (remaining as u64, remaining);
        }
    }

    (KV_SCORES_TTL_SECONDS, IN_MEMORY_TTL_SECONDS)
}

fn default_kv_ttl_seconds() -> i64 {
    KV_SCORES_TTL_SECONDS as i64
}
