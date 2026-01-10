use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::model::score::Statistic;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Bettors {
    pub bettor_name: String,
    pub total_score: i32,
    pub scoreboard_position_name: String,
    pub scoreboard_position: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Scores {
    pub eup_id: i64,
    pub espn_id: i64,
    pub golfer_name: String,
    pub bettor_name: String,
    pub detailed_statistics: Statistic,
    pub group: i64,
    pub score_view_step_factor: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScoresAndLastRefresh {
    pub score_struct: Vec<Scores>,
    pub last_refresh: NaiveDateTime,
    pub last_refresh_source: RefreshSource,
}

pub enum CheckType {
    Table,
    Constraint,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScoreData {
    pub bettor_struct: Vec<Bettors>,
    pub score_struct: Vec<Scores>,
    pub last_refresh: String,
    pub last_refresh_source: RefreshSource,
    pub cache_hit: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RefreshSource {
    Db,
    Espn,
}

impl fmt::Display for RefreshSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            RefreshSource::Db => "database",
            RefreshSource::Espn => "ESPN",
        };
        write!(f, "{s}")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BettorScoreByRound {
    pub bettor_name: String,
    pub computed_rounds: Vec<isize>,
    pub scores_aggregated_by_golf_grp_by_rd: Vec<isize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AllBettorScoresByRound {
    pub summary_scores: Vec<BettorScoreByRound>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DetailedScore {
    pub bettor_name: String,
    pub golfer_name: String,
    pub golfer_espn_id: i64,
    pub rounds: Vec<i32>,
    pub scores: Vec<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SummaryDetailedScores {
    pub detailed_scores: Vec<DetailedScore>,
}
