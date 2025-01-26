// use actix_web::cookie::time::format_description::well_known::iso8601::Config;
// use deadpool_postgres::tokio_postgres::Row;
use serde::{Deserialize, Serialize};
use sqlx_middleware::middleware::{ConfigAndPool, MiddlewarePool, MiddlewarePoolConnection};
use sqlx_middleware::SqlMiddlewareDbError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use sqlx_middleware::db::{Db, QueryState};
use sqlx_middleware::middleware::{QueryAndParams as QueryAndParams2, RowValues as RowValues2};
use sqlx_middleware::model::{DatabaseResult, QueryAndParams, RowValues};

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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Statistic {
    pub eup_id: i64,
    pub rounds: Vec<IntStat>,
    pub round_scores: Vec<IntStat>,
    pub tee_times: Vec<StringStat>,
    pub holes_completed_by_round: Vec<IntStat>,
    pub line_scores: Vec<LineScore>,
    pub success_fail: ResultStatus,
    pub total_score: i32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum ResultStatus {
    NoData,
    NoDisplayValue,
    Success,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StringStat {
    pub val: String,
    pub success: ResultStatus,
    // pub last_refresh_date: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IntStat {
    pub val: i32,
    pub success: ResultStatus,
    // pub last_refresh_date: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LineScore {
    pub round: i32,
    pub hole: i32,
    pub score: i32,
    pub par: i32,
    pub score_display: ScoreDisplay,
    pub success: ResultStatus,
    // pub last_refresh_date: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ScoreDisplay {
    DoubleCondor,
    Condor,
    Albatross,
    Eagle,
    Birdie,
    Par,
    Bogey,
    DoubleBogey,
    TripleBogey,
    QuadrupleBogey,
    QuintupleBogey,
    SextupleBogey,
    SeptupleBogey,
    OctupleBogey,
    NonupleBogey,
    DodecupleBogey,
}

impl ScoreDisplay {
    pub fn from_i32(i: i32) -> Self {
        match i {
            -5 => ScoreDisplay::DoubleCondor,
            -4 => ScoreDisplay::Condor,
            -3 => ScoreDisplay::Albatross,
            -2 => ScoreDisplay::Eagle,
            -1 => ScoreDisplay::Birdie,
            0 => ScoreDisplay::Par,
            1 => ScoreDisplay::Bogey,
            2 => ScoreDisplay::DoubleBogey,
            3 => ScoreDisplay::TripleBogey,
            4 => ScoreDisplay::QuadrupleBogey,
            5 => ScoreDisplay::QuintupleBogey,
            6 => ScoreDisplay::SextupleBogey,
            7 => ScoreDisplay::SeptupleBogey,
            8 => ScoreDisplay::OctupleBogey,
            9 => ScoreDisplay::NonupleBogey,
            10 => ScoreDisplay::DodecupleBogey,
            _ => ScoreDisplay::Par,
        }
    }
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
}

pub struct BettorScoreByRound {
    pub bettor_name: String,
    pub computed_rounds: Vec<isize>,
    pub scores_aggregated_by_golf_grp_by_rd: Vec<isize>,
}

pub struct AllBettorScoresByRound {
    pub summary_scores: Vec<BettorScoreByRound>,
}

// New Data Structures for the Function's Output
#[derive(Serialize, Deserialize, Debug)]
pub struct DetailedScore {
    pub bettor_name: String,
    pub golfer_name: String,
    pub rounds: Vec<i32>,
    pub scores: Vec<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SummaryDetailedScores {
    pub detailed_scores: Vec<DetailedScore>,
}

pub const TABLES_AND_DDL: &[(&str, &str, &str, &str)] = &[
    (
        "event",
        include_str!("admin/model/sql/schema/postgres/00_event.sql"),
        "",
        "",
    ),
    (
        "golfstatistic",
        include_str!("admin/model/sql/schema/postgres/01_golfstatistic.sql"),
        "",
        "",
    ),
    (
        "player",
        include_str!("admin/model/sql/schema/postgres/02_player.sql"),
        "",
        "",
    ),
    (
        "golfuser",
        include_str!("admin/model/sql/schema/postgres/03_golfuser.sql"),
        "",
        "",
    ),
    (
        "event_user_player",
        include_str!("admin/model/sql/schema/postgres/04_event_user_player.sql"),
        "",
        "",
    ),
    (
        "eup_statistic",
        include_str!("admin/model/sql/schema/postgres/05_eup_statistic.sql"),
        "",
        "",
    ),
];

pub const TABLES_CONSTRAINT_TYPE_CONSTRAINT_NAME_AND_DDL: &[(&str, &str, &str, &str)] = &[
    (
        "player",
        "UNIQUE",
        "unq_name",
        include_str!("admin/model/sql/constraints/01_player.sql"),
    ),
    (
        "player",
        "UNIQUE",
        "unq_espn_id",
        include_str!("admin/model/sql/constraints/02_player.sql"),
    ),
    (
        "event_user_player",
        "UNIQUE",
        "unq_event_id_user_id_player_id",
        include_str!("admin/model/sql/constraints/03_event_user_player.sql"),
    ),
];

pub type CacheMap = Arc<RwLock<HashMap<String, Cache>>>;

pub async fn get_golfers_from_db(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<Vec<Scores>, SqlMiddlewareDbError> {
    let pool = config_and_pool.pool.get().await.unwrap();
    let conn = MiddlewarePool::get_connection(pool).await.unwrap();
    let query = match &conn {
        MiddlewarePoolConnection::Postgres(_) => {
            "SELECT grp, golfername, playername, eup_id, espn_id FROM sp_get_player_names($1) ORDER BY grp, eup_id"
        }
        MiddlewarePoolConnection::Sqlite(_) => {
            include_str!("admin/model/sql/functions/sqlite/02_sp_get_player_names.sql")
        }
    };

    let query_and_params = QueryAndParams2 {
        query: query.to_string(),
        params: vec![RowValues2::Int(event_id as i64)],
        is_read_only: true,
    };

    let res = match &conn {
        MiddlewarePoolConnection::Sqlite(sconn) => {
            // let conn = conn.lock().unwrap();
            sconn
                .interact(move |xxx| {
                    let converted_params =
                        sqlx_middleware::sqlite_convert_params(&query_and_params.params)?;
                    let tx = xxx.transaction()?;

                    let result_set = {
                        let mut stmt = tx.prepare(&query_and_params.query)?;
                        let rs =
                            sqlx_middleware::sqlite_build_result_set(&mut stmt, &converted_params)?;
                        rs
                    };
                    tx.commit()?;
                    Ok::<_, SqlMiddlewareDbError>(result_set)
                })
                .await
        }
        _ => panic!("Only sqlite is supported "),
    }?;

    let z = res?
        .results
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
                .get("bettorname")
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
                round_scores: vec![],
                tee_times: vec![],
                holes_completed_by_round: vec![],
                line_scores: vec![],
                success_fail: ResultStatus::NoData,
                total_score: 0,
            },
        })
        .collect();

    Ok(z)
}

pub async fn get_title_from_db(
    db: &Db,
    event_id: i32,
) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
    // let query = "SELECT eventname FROM sp_get_event_name($1)";
    let query: &str = if db.config_and_pool.db_type == sqlx_middleware::db::DatabaseType::Postgres {
        "SELECT eventname FROM sp_get_event_name($1)"
    } else {
        include_str!("admin/model/sql/functions/sqlite/01_sp_get_event_name.sql")
    };
    let query_and_params = QueryAndParams {
        query: query.to_string(),
        params: vec![RowValues::Int(event_id as i64)],
    };
    let result = db.exec_general_query(vec![query_and_params], true).await;

    let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();

    let missing_tables = match result {
        Ok(r) => {
            dbresult.db_last_exec_state = r.db_last_exec_state;
            dbresult.error_message = r.error_message;
            r.return_result[0].results.clone()
        }
        Err(e) => {
            let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
            let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();
            dbresult.error_message = Some(emessage);
            vec![]
        }
    };

    let zz: Vec<_> = missing_tables
        .iter()
        .filter_map(|row| {
            let exists_index = row.column_names.iter().position(|col| col == "eventname")?;

            match &row.rows[exists_index] {
                RowValues::Text(value) => Some(value),

                _ => None,
            }
        })
        .collect();
    if !zz.is_empty() {
        dbresult.return_result = zz[0].to_string();
    }
    Ok(dbresult)
}

pub async fn get_scores_from_db(
    db: &Db,
    event_id: i32,
) -> Result<DatabaseResult<Vec<Scores>>, Box<dyn std::error::Error>> {
    let query: &str = if db.config_and_pool.db_type == sqlx_middleware::db::DatabaseType::Postgres {
        "SELECT * FROM sp_get_scores($1)"
    } else {
        include_str!("admin/model/sql/functions/sqlite/03_sp_get_scores.sql")
    };
    let query_and_params = QueryAndParams {
        query: query.to_string(),
        params: vec![RowValues::Int(event_id as i64)],
    };
    let result = db.exec_general_query(vec![query_and_params], true).await;

    let mut dbresult: DatabaseResult<Vec<Scores>> = DatabaseResult::<Vec<Scores>>::default();

    match result {
        Ok(r) => {
            dbresult.db_last_exec_state = r.db_last_exec_state;
            dbresult.error_message = r.error_message;
            if r.db_last_exec_state == QueryState::QueryReturnedSuccessfully {
                let rows = &r.return_result[0].results;
                let scores = rows
                    .iter()
                    .map(|row| Scores {
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
                        golfer_name: row
                            .get("golfername")
                            .and_then(|v| v.as_text())
                            .unwrap_or_default()
                            .to_string(),
                        group: row
                            .get("grp")
                            .and_then(|v| v.as_int())
                            .copied()
                            .unwrap_or_default(),

                        bettor_name: row
                            .get("bettor")
                            .and_then(|v| v.as_text())
                            .unwrap_or_default()
                            .to_string(),
                        detailed_statistics: Statistic {
                            eup_id: row
                                .get("eup_id")
                                .and_then(|v| v.as_int())
                                .copied()
                                .unwrap_or_default(),
                            rounds: vec![],
                            round_scores: vec![],
                            tee_times: vec![],
                            holes_completed_by_round: vec![],
                            line_scores: vec![],
                            success_fail: ResultStatus::NoData,
                            total_score: 0,
                        },
                    })
                    .collect();
                dbresult.return_result = scores;
            }
        }
        Err(e) => {
            let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
            dbresult.error_message = Some(emessage);
        }
    }

    Ok(dbresult)
}

pub async fn store_scores_in_db(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
    scores: &Vec<Scores>,
) -> Result<(), SqlMiddlewareDbError> {
    fn build_insert_stms(scores: &Vec<Scores>, event_id: i32) -> Vec<QueryAndParams2> {
        let mut queries = vec![];
        for score in scores {
            let insert_stmt =
                include_str!("admin/model/sql/functions/sqlite/04_sp_set_eup_statistic.sql");
            let param = vec![
                RowValues2::Int(event_id as i64),
                RowValues2::Int(score.eup_id),
                RowValues2::Int(score.group),
                RowValues2::Text(
                    serde_json::to_string(score.detailed_statistics.rounds.as_slice()).unwrap(),
                ),
                RowValues2::Text(
                    serde_json::to_string(score.detailed_statistics.round_scores.as_slice())
                        .unwrap(),
                ),
                RowValues2::Text(
                    serde_json::to_string(score.detailed_statistics.tee_times.as_slice()).unwrap(),
                ),
                RowValues2::Text(
                    serde_json::to_string(
                        score
                            .detailed_statistics
                            .holes_completed_by_round
                            .as_slice(),
                    )
                    .unwrap(),
                ),
                RowValues2::Text(
                    serde_json::to_string(score.detailed_statistics.line_scores.as_slice())
                        .unwrap(),
                ),
            ];
            queries.push(QueryAndParams2 {
                query: insert_stmt.to_string(),
                params: param,
                is_read_only: false,
            });
        }
        queries
    }

    let pool = config_and_pool.pool.get().await.unwrap();
    let conn = MiddlewarePool::get_connection(pool).await.unwrap();
    let queries = build_insert_stms(scores, event_id);

    match &conn {
        MiddlewarePoolConnection::Sqlite(sconn) => {
            // let conn = conn.lock().unwrap();
            sconn
                .interact(move |xxx| {
                    let tx = xxx.transaction()?;
                    {
                        let mut stmt = tx.prepare(&queries[0].query)?;
                        for query in queries {
                            let converted_params =
                                sqlx_middleware::sqlite_convert_params(&query.params)?;

                            let _rs = stmt.execute(sqlx_middleware::sqlite_params_from_iter(
                                converted_params.iter(),
                            ))?;
                        }
                    }
                    tx.commit()?;
                    Ok::<_, SqlMiddlewareDbError>(())
                })
                .await??;
            Ok(())
        }
        _ => panic!("Only sqlite is supported "),
    }
}
