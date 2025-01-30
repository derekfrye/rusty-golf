use chrono::NaiveDateTime;
// use deadpool_postgres::tokio_postgres::Row;
// use actix_web::cookie::time::format_description::well_known::iso8601::Config;
// use deadpool_postgres::tokio_postgres::Row;
use serde::{Deserialize, Serialize};
use sql_middleware::middleware::{ConfigAndPool, MiddlewarePool, MiddlewarePoolConnection};
use sql_middleware::SqlMiddlewareDbError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// use sqlx_middleware::db::Db;
use sql_middleware::middleware::{QueryAndParams as QueryAndParams2, RowValues as RowValues2};
// use sqlx_middleware::model::{DatabaseResult, QueryAndParams, RowValues};

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
                        sql_middleware::sqlite_convert_params(&query_and_params.params)?;
                    let tx = xxx.transaction()?;

                    let result_set = {
                        let mut stmt = tx.prepare(&query_and_params.query)?;
                        let rs =
                            sql_middleware::sqlite_build_result_set(&mut stmt, &converted_params)?;
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
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<String, SqlMiddlewareDbError> {
    let pool = config_and_pool.pool.get().await.unwrap();
    let conn = MiddlewarePool::get_connection(pool).await.unwrap();

    let query = match &conn {
        MiddlewarePoolConnection::Postgres(_) => {
            "SELECT EXISTS(SELECT 1 FROM event WHERE event_id = $1)"
        }
        MiddlewarePoolConnection::Sqlite(_) => {
            include_str!("admin/model/sql/functions/sqlite/01_sp_get_event_name.sql")
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
                        sql_middleware::sqlite_convert_params(&query_and_params.params)?;
                    let tx = xxx.transaction()?;

                    let result_set = {
                        let mut stmt = tx.prepare(&query_and_params.query)?;
                        let rs =
                            sql_middleware::sqlite_build_result_set(&mut stmt, &converted_params)?;
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
        .map(|row| {
            row.get("name")
                .and_then(|v| v.as_text())
                .map(|v| v.to_string())
                .ok_or(SqlMiddlewareDbError::Other("Name not found".to_string()))
        })
        .last()
        .unwrap_or_else(|| Err(SqlMiddlewareDbError::Other("No results found".to_string())));

    z
}

pub async fn get_scores_from_db(
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
            include_str!("admin/model/sql/functions/sqlite/03_sp_get_scores.sql")
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
                        sql_middleware::sqlite_convert_params(&query_and_params.params)?;
                    let tx = xxx.transaction()?;

                    let result_set = {
                        let mut stmt = tx.prepare(&query_and_params.query)?;
                        let rs =
                            sql_middleware::sqlite_build_result_set(&mut stmt, &converted_params)?;
                        rs
                    };
                    tx.commit()?;
                    Ok::<_, SqlMiddlewareDbError>(result_set)
                })
                .await
        }
        _ => panic!("Only sqlite is supported "),
    }?;

    let z: Result<Vec<Scores>, SqlMiddlewareDbError> = res?
        .results
        .iter()
        .map(|row| {
            Ok(Scores {
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
                    .get("golfer_espn_id")
                    .and_then(|v| v.as_int())
                    .copied()
                    .unwrap_or_default(),
                detailed_statistics: Statistic {
                    eup_id: row
                        .get("eup_id")
                        .and_then(|v| v.as_int())
                        .copied()
                        .unwrap_or_default(),
                    rounds: match serde_json::from_str(
                        row.get("rounds")
                            .and_then(|v| v.as_text())
                            .unwrap_or_default(),
                    ) {
                        Ok(rounds) => rounds,
                        Err(e) => return Err(SqlMiddlewareDbError::Other(e.to_string())),
                    },
                    round_scores: match serde_json::from_str(
                        row.get("round_scores")
                            .and_then(|v| v.as_text())
                            .unwrap_or_default(),
                    ) {
                        Ok(round_scores) => round_scores,
                        Err(e) => return Err(SqlMiddlewareDbError::Other(e.to_string())),
                    },
                    tee_times: match serde_json::from_str(
                        row.get("tee_times")
                            .and_then(|v| v.as_text())
                            .unwrap_or_default(),
                    ) {
                        Ok(tee_times) => tee_times,
                        Err(e) => return Err(SqlMiddlewareDbError::Other(e.to_string())),
                    },
                    holes_completed_by_round: match serde_json::from_str(
                        row.get("holes_completed_by_round")
                            .and_then(|v| v.as_text())
                            .unwrap_or_default(),
                    ) {
                        Ok(holes_completed_by_round) => holes_completed_by_round,
                        Err(e) => return Err(SqlMiddlewareDbError::Other(e.to_string())),
                    },
                    line_scores: match serde_json::from_str(
                        row.get("line_scores")
                            .and_then(|v| v.as_text())
                            .unwrap_or_default(),
                    ) {
                        Ok(line_scores) => line_scores,
                        Err(e) => return Err(SqlMiddlewareDbError::Other(e.to_string())),
                    },
                    success_fail: ResultStatus::Success,
                    total_score: row
                        .get("total_score")
                        .and_then(|v| v.as_int())
                        .map(|v| *v as i32)
                        .unwrap_or_default(),
                },
            })
        })
        .collect::<Result<Vec<Scores>, SqlMiddlewareDbError>>();

    Ok(z?)
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
                RowValues2::Int(score.espn_id),
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
                RowValues2::Int(score.detailed_statistics.total_score as i64),
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
            sconn
                .interact(move |xxx| {
                    let tx = xxx.transaction()?;
                    {
                        // println!("Query: {:?}", queries[0].query);
                        let mut stmt = tx.prepare(&queries[0].query)?;
                        let x = stmt.expanded_sql();

                        if cfg!(debug_assertions) {
                            println!("Query from dbg: {:?}", x);
                        }
                        for query in queries {
                            let converted_params =
                                sql_middleware::sqlite_convert_params(&query.params)?;

                            let _rs = stmt.execute(sql_middleware::sqlite_params_from_iter(
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

pub async fn event_and_scores_already_in_db(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
    cache_max_age: i64,
) -> Result<bool, SqlMiddlewareDbError> {
    let z = get_title_from_db(config_and_pool, event_id).await;
    // if this threw an error, the event isn't setup, so clearly we can early return since we need to retrieve the stuff from espn
    match z {
        Err(_) => {
            return Ok(false);
        }
        Ok(_) => {}
    }

    let pool = config_and_pool.pool.get().await.unwrap();
    let conn = MiddlewarePool::get_connection(pool).await.unwrap();
    let query = match &conn {
        MiddlewarePoolConnection::Postgres(_) => {
            "SELECT EXISTS(SELECT 1 FROM event WHERE event_id = $1)"
        }
        MiddlewarePoolConnection::Sqlite(_) => {
            include_str!(
                "admin/model/sql/functions/sqlite/05_sp_get_event_and_scores_already_in_db.sql"
            )
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
                        sql_middleware::sqlite_convert_params(&query_and_params.params)?;
                    let tx = xxx.transaction()?;

                    let result_set = {
                        let mut stmt = tx.prepare(&query_and_params.query)?;
                        let rs =
                            sql_middleware::sqlite_build_result_set(&mut stmt, &converted_params)?;
                        rs
                    };
                    tx.commit()?;
                    Ok::<_, SqlMiddlewareDbError>(result_set)
                })
                .await
        }
        _ => panic!("Only sqlite is supported "),
    }?;

    let z: Result<NaiveDateTime, SqlMiddlewareDbError> = res?
        .results
        .iter()
        .map(|row| {
            Ok(row
                .get("ins_ts")
                .and_then(|v| v.as_timestamp())
                .unwrap_or_default())
        })
        .last()
        .ok_or(SqlMiddlewareDbError::Other("No results found".to_string()))?;

    let now = chrono::Utc::now().naive_utc();
    let diff = now - z?;
    Ok(diff.num_days() < cache_max_age)
}
