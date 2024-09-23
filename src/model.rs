use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize, Clone)]
pub struct Bettors {
    pub bettor_name: String,
    pub total_score: i32,
    pub scoreboard_position_name: String,
    pub scoreboard_position: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Scores {
    pub eup_id: i64,
    pub espn_id: i64,
    pub golfer_name: String,
    pub bettor_name: String,
    pub detailed_statistics: Statistic,
    pub group: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Statistic {
    pub eup_id: i64,
    pub rounds: Vec<IntStat>,
    pub scores: Vec<IntStat>,
    pub tee_times: Vec<StringStat>,
    pub holes_completed: Vec<IntStat>,
    pub success_fail: ResultStatus,
    pub total_score: i32,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum ResultStatus {
    NoData,
    NoDisplayValue,
    Success,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StringStat {
    pub val: String,
    pub success: ResultStatus,
    pub last_refresh_date: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IntStat {
    pub val: i32,
    pub success: ResultStatus,
    pub last_refresh_date: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerJsonResponse {
    pub data: Vec<HashMap<String, serde_json::Value>>,
    pub eup_ids: Vec<i64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Cache {
    pub data: Option<ScoreData>,
    pub cached_time: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ScoreData {
    pub bettor_struct: Vec<Bettors>,
    pub score_struct: Vec<Scores>,
    pub last_refresh: String,
}

pub type CacheMap = Arc<RwLock<HashMap<String, Cache>>>;
