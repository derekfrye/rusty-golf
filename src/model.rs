use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::db::db::{DatabaseResult, DatabaseSetupState, Db, QueryAndParams, RowValues};

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

pub struct SummaryScore {
    pub bettor_name: String,
    pub computed_rounds: Vec<isize>,
    pub new_scores: Vec<isize>,
}

pub struct SummaryScores {
    pub summary_scores: Vec<SummaryScore>,
}

pub type CacheMap = Arc<RwLock<HashMap<String, Cache>>>;

pub async fn get_golfers_from_db(
    db: &Db,
    event_id: i32,
) -> Result<DatabaseResult<Vec<Scores>>, Box<dyn std::error::Error>> {
    let query =
        "SELECT grp, golfername, playername, eup_id, espn_id FROM sp_get_player_names($1) ORDER BY grp, eup_id";
    let query_and_params = QueryAndParams {
        query: query.to_string(),
        params: vec![RowValues::Int(event_id as i64)],
    };
    let result = db.exec_general_query(vec![query_and_params], true).await;
    let mut dbresult: DatabaseResult<Vec<Scores>> = DatabaseResult {
        db_last_exec_state: DatabaseSetupState::QueryReturnedSuccessfully,
        return_result: vec![],
        error_message: None,
        db_object_name: "sp_get_player_names".to_string(),
    };

    match result {
        Ok(r) => {
            if r.db_last_exec_state == DatabaseSetupState::QueryReturnedSuccessfully {
                let rows = r.return_result[0].results.clone();
                let players = rows
                    .iter()
                    .map(|row| Scores {
                        // parse column 0 as an int32
                        group: row
                            .get("grp")
                            .and_then(|v| v.as_int())
                            .copied()
                            .unwrap_or_default(),
                        golfer_name: row
                            .get("golfername")
                            .and_then(|v| v.as_text())
                            .unwrap_or_default()
                            .to_string(),
                        bettor_name: row
                            .get("playername")
                            .and_then(|v| v.as_text())
                            .unwrap_or_default()
                            .to_string(),
                        eup_id: row
                            .get("eup_id")
                            .and_then(|v| v.as_int())
                            .copied()
                            .unwrap_or_default(),
                        espn_id: row
                            .get("espn_id")
                            .and_then(|v| v.as_int())
                            .copied()
                            .unwrap_or_default(),
                        detailed_statistics: Statistic {
                            eup_id: row
                                .get("eup_id")
                                .and_then(|v| v.as_int())
                                .copied()
                                .unwrap_or_default(),
                            rounds: vec![],
                            scores: vec![],
                            tee_times: vec![],
                            holes_completed: vec![],
                            success_fail: ResultStatus::NoData,
                            total_score: 0,
                        },
                    })
                    .collect();
                dbresult.return_result = players;
            }
        }
        Err(e) => {
            let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
            dbresult.error_message = Some(emessage);
        }
    }

    Ok(dbresult)
}
