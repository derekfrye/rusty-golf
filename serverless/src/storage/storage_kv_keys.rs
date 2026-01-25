#![cfg(target_arch = "wasm32")]

use crate::storage::ServerlessStorage;

impl ServerlessStorage {
    pub fn scores_key(event_id: i32) -> String {
        format!("events/{event_id}/scores.json")
    }

    pub fn espn_cache_key(event_id: i32) -> String {
        format!("cache/espn/{event_id}.json")
    }

    pub fn kv_event_details_key(event_id: i32) -> String {
        format!("event:{event_id}:details")
    }

    pub fn kv_scores_cache_key(event_id: i32) -> String {
        format!("event:{event_id}:scores_cache")
    }

    pub fn kv_golfers_key(event_id: i32) -> String {
        format!("event:{event_id}:golfers")
    }

    pub fn kv_player_factors_key(event_id: i32) -> String {
        format!("event:{event_id}:player_factors")
    }

    pub fn kv_last_refresh_key(event_id: i32) -> String {
        format!("event:{event_id}:last_refresh")
    }

    pub fn kv_seeded_at_key(event_id: i32, suffix: &str) -> String {
        format!("event:{event_id}:{suffix}:seeded_at")
    }

    pub fn kv_force_espn_fail_key(event_id: i32) -> String {
        format!("event:{event_id}:force_espn_fail")
    }

    pub fn kv_test_lock_key(event_id: i32) -> String {
        format!("event:{event_id}:test_lock")
    }

    pub fn kv_test_lock_prefix() -> &'static str {
        "event:"
    }
}
