#![cfg(target_arch = "wasm32")]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use rusty_golf_core::model::RefreshSource;
use rusty_golf_core::model::Scores;

#[derive(Clone)]
pub struct EventListing {
    pub event_id: i32,
    pub event_name: String,
    pub score_view_step_factor: f32,
    pub refresh_from_espn: i64,
}

#[derive(Serialize, Deserialize)]
pub struct EventDetailsDoc {
    pub event_name: String,
    pub score_view_step_factor: f32,
    pub refresh_from_espn: i64,
    pub end_date: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct GolferAssignment {
    pub eup_id: i64,
    pub espn_id: i64,
    pub golfer_name: String,
    pub bettor_name: String,
    pub group: i64,
    pub score_view_step_factor: Option<f32>,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerFactorEntry {
    pub golfer_espn_id: i64,
    pub bettor_name: String,
    pub step_factor: f32,
}

#[derive(Serialize, Deserialize)]
pub struct AuthTokensDoc {
    pub tokens: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct LastRefreshDoc {
    pub ts: String,
    pub source: RefreshSource,
}

#[derive(Serialize, Deserialize)]
pub struct SeededAtDoc {
    pub seeded_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct TestLockDoc {
    pub shared_holders: HashMap<String, String>,
    pub exclusive_holder: Option<(String, String)>,
}

#[derive(Clone, Copy)]
pub enum TestLockMode {
    Shared,
    Exclusive,
}

#[derive(Debug, Deserialize)]
pub struct AdminSeedRequest {
    pub event_id: i32,
    pub refresh_from_espn: i64,
    pub event: AdminEupEvent,
    pub score_struct: Vec<Scores>,
    pub espn_cache: serde_json::Value,
    pub auth_tokens: Option<Vec<String>>,
    pub last_refresh: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminEupEvent {
    pub event: i64,
    pub name: String,
    pub score_view_step_factor: serde_json::Value,
    pub end_date: Option<String>,
    pub data_to_fill_if_event_and_year_missing: Vec<AdminEupDataFill>,
}

#[derive(Debug, Deserialize)]
pub struct AdminEupDataFill {
    pub golfers: Vec<AdminEupGolfer>,
    pub event_user_player: Vec<AdminEupEventUserPlayer>,
}

#[derive(Debug, Deserialize)]
pub struct AdminEupGolfer {
    pub espn_id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminEupEventUserPlayer {
    pub bettor: String,
    pub golfer_espn_id: i64,
    pub score_view_step_factor: Option<serde_json::Value>,
}
