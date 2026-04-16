#![cfg(target_arch = "wasm32")]

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AdminCleanupRequest {
    pub event_id: i32,
    #[serde(default)]
    pub include_auth_tokens: bool,
}

#[derive(Deserialize)]
pub struct AdminCleanupScoresRequest {
    pub event_id: i32,
}

#[derive(Deserialize)]
pub struct AdminCacheFlushRequest {
    pub event_id: i32,
}

#[derive(Deserialize)]
pub struct AdminUpdateDatesRequest {
    pub event_id: i32,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub completed: Option<bool>,
}

#[derive(Deserialize)]
pub struct AdminEspnFailRequest {
    pub event_id: i32,
    pub enabled: bool,
}

#[derive(Deserialize)]
pub struct AdminTestLockRequest {
    pub event_id: i32,
    pub token: String,
    pub ttl_secs: Option<i64>,
    pub mode: Option<String>,
    #[serde(default)]
    pub force: bool,
}

#[derive(Serialize)]
pub struct AdminTestLockResponse {
    pub acquired: bool,
    pub is_first: bool,
}

#[derive(Deserialize)]
pub struct AdminTestUnlockRequest {
    pub event_id: AdminEventSelector,
    pub token: String,
}

#[derive(Serialize)]
pub struct AdminTestUnlockResponse {
    pub is_last: bool,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum AdminEventSelector {
    All(String),
    Id(i32),
}
