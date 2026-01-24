#![cfg(target_arch = "wasm32")]

use chrono::Utc;
use once_cell::sync::Lazy;
use rusty_golf_core::model::ScoresAndLastRefresh;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

pub const KV_SCORES_TTL_SECONDS: u64 = 300;
const IN_MEMORY_TTL_SECONDS: i64 = 30;

#[derive(Serialize, Deserialize, Clone)]
pub struct KvScoresCacheEntry {
    pub cached_at: i64,
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
}

static IN_MEMORY_SCORES: Lazy<Mutex<HashMap<i32, CacheEntry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn get_in_memory_scores(event_id: i32) -> Option<ScoresAndLastRefresh> {
    let now = Utc::now().timestamp();
    let mut cache = IN_MEMORY_SCORES.lock().ok()?;
    if let Some(entry) = cache.get(&event_id) {
        if now - entry.inserted_at <= IN_MEMORY_TTL_SECONDS {
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
    if age > IN_MEMORY_TTL_SECONDS {
        cache.remove(&event_id);
        return CacheStatus {
            exists: false,
            remaining_ttl_seconds: None,
        };
    }
    let remaining = (IN_MEMORY_TTL_SECONDS - age).max(0);
    CacheStatus {
        exists: true,
        remaining_ttl_seconds: Some(remaining),
    }
}

pub fn set_in_memory_scores(event_id: i32, scores: &ScoresAndLastRefresh) {
    let now = Utc::now().timestamp();
    let Ok(mut cache) = IN_MEMORY_SCORES.lock() else {
        return;
    };
    cache.insert(
        event_id,
        CacheEntry {
            value: scores.clone(),
            inserted_at: now,
        },
    );
}

pub fn clear_in_memory_scores(event_id: i32) {
    let Ok(mut cache) = IN_MEMORY_SCORES.lock() else {
        return;
    };
    cache.remove(&event_id);
}

pub fn build_kv_scores_entry(scores: &ScoresAndLastRefresh) -> KvScoresCacheEntry {
    KvScoresCacheEntry {
        cached_at: Utc::now().timestamp(),
        payload: scores.clone(),
    }
}

pub fn parse_kv_scores_entry(
    text: &str,
) -> Result<(ScoresAndLastRefresh, Option<i64>), serde_json::Error> {
    if let Ok(entry) = serde_json::from_str::<KvScoresCacheEntry>(text) {
        let remaining = kv_remaining_ttl_seconds(entry.cached_at);
        return Ok((entry.payload, Some(remaining)));
    }
    let fallback = serde_json::from_str::<ScoresAndLastRefresh>(text)?;
    Ok((fallback, None))
}

pub fn kv_remaining_ttl_seconds(cached_at: i64) -> i64 {
    let now = Utc::now().timestamp();
    let age = now - cached_at;
    let remaining = (KV_SCORES_TTL_SECONDS as i64) - age;
    remaining.max(0)
}
