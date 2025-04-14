use chrono::{NaiveDateTime, Duration as ChronoDuration};
use sql_middleware::middleware::ResultSet;
// use deadpool_postgres::tokio_postgres::Row;
// use actix_web::cookie::time::format_description::well_known::iso8601::Config;
// use deadpool_postgres::tokio_postgres::Row;
use serde::{Deserialize, Serialize};
use sql_middleware::middleware::{
    ConfigAndPool, ConversionMode, MiddlewarePool, MiddlewarePoolConnection
};
use sql_middleware::{
    convert_sql_params, SqlMiddlewareDbError, SqliteParamsExecute, SqliteParamsQuery,
};
use std::collections::HashMap;

// use sqlx_middleware::db::Db;
use sql_middleware::middleware::{QueryAndParams as QueryAndParams2, RowValues as RowValues2};
use std::fmt;
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
pub struct ScoresAndLastRefresh {
    pub score_struct: Vec<Scores>,
    pub last_refresh: NaiveDateTime,
    pub last_refresh_source: RefreshSource,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Statistic {
    pub eup_id: i64,
    pub rounds: Vec<IntStat>,
    pub round_scores: Vec<IntStat>,
    pub tee_times: Vec<StringStat>,
    pub holes_completed_by_round: Vec<IntStat>,
    pub line_scores: Vec<LineScore>,
    pub total_score: i32,
}

pub enum CheckType {
    Table,
    Constraint,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StringStat {
    pub val: String,
    // pub last_refresh_date: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IntStat {
    pub val: i32,
    // pub last_refresh_date: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LineScore {
    pub round: i32,
    pub hole: i32,
    pub score: i32,
    pub par: i32,
    pub score_display: ScoreDisplay,
    // pub last_refresh_date: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
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

// Add proper From implementation for i32
impl From<i32> for ScoreDisplay {
    fn from(value: i32) -> Self {
        Self::from_i32(value)
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
    pub last_refresh_source: RefreshSource,
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
        write!(f, "{}", s)
    }
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

// pub type CacheMap = Arc<RwLock<HashMap<String, Cache>>>;

pub fn format_time_ago_for_score_view(td: ChronoDuration) -> String {
    // Get the total number of seconds from the TimeDelta.
    let secs = td.num_seconds();

    // Define approximate durations for each time unit.
    const MINUTE: i64 = 60;
    const HOUR: i64 = 60 * MINUTE; // 3600 seconds
    const DAY: i64 = 24 * HOUR; // 86400 seconds
    const WEEK: i64 = 7 * DAY; // 604800 seconds
    const MONTH: i64 = 30 * DAY; // 2592000 seconds (approximation)
    const YEAR: i64 = 365 * DAY; // 31536000 seconds (approximation)

    // Choose the largest fitting time unit.
    if secs >= YEAR {
        // Use floating point division to capture partial years.
        let years = (secs as f64) / (YEAR as f64);
        if years == 1.0 {
            "1 year".to_string()
        } else {
            format!("{:.2} years", years)
        }
    } else if secs >= MONTH {
        let months = (secs as f64) / (MONTH as f64);
        format!("{:.2} months", months)
    } else if secs >= WEEK {
        let weeks = secs / WEEK;
        if weeks == 1 {
            "1 week".to_string()
        } else {
            format!("{} weeks", weeks)
        }
    } else if secs >= DAY {
        let days = secs / DAY;
        if days == 1 {
            "1 day".to_string()
        } else {
            format!("{} days", days)
        }
    } else if secs >= HOUR {
        let hours = secs / HOUR;
        if hours == 1 {
            "1 hour".to_string()
        } else {
            format!("{} hours", hours)
        }
    } else if secs >= MINUTE {
        let minutes = secs / MINUTE;
        if minutes == 1 {
            "1 minute".to_string()
        } else {
            format!("{} minutes", minutes)
        }
    } else {
        // For less than one minute, report seconds.
        if secs == 1 {
            "1 second".to_string()
        } else {
            format!("{} seconds", secs)
        }
    }
}

/// Removes the last character from a string
/// 
/// # Arguments
/// * `s` - The input string
/// 
/// # Returns
/// A new string with the last character removed, or an empty string if input is empty
pub fn take_a_char_off(s: &str) -> String {
    // This avoids unnecessary allocation when we need to remove the last character
    let mut result = s.to_string();
    result.pop(); // Safely handles empty strings (does nothing if empty)
    result
}

/// Helper function to parse JSON from a field in a row
fn parse_json_field<T>(row: &sql_middleware::middleware::CustomDbRow, field_name: &str) -> Result<T, SqlMiddlewareDbError> 
where
    T: for<'de> serde::Deserialize<'de> 
{
    let json_text = row
        .get(field_name)
        .and_then(|v| v.as_text())
        .unwrap_or_default();
        
    serde_json::from_str(json_text)
        .map_err(|e| SqlMiddlewareDbError::Other(format!("Failed to parse {} field: {}", field_name, e)))
}

/// Helper function to get the last timestamp from the result set
fn get_last_timestamp(results: &[sql_middleware::middleware::CustomDbRow]) -> chrono::NaiveDateTime {
    results
        .iter()
        .filter_map(|row| row.get("ins_ts").and_then(|v| v.as_timestamp()))
        .last()
        .unwrap_or_else(|| chrono::Utc::now().naive_utc())
}

/// Helper function to execute a SQL query with params using the appropriate database connection
async fn execute_query(
    conn: &MiddlewarePoolConnection,
    query: &str, 
    params: Vec<RowValues2>
) -> Result<ResultSet, SqlMiddlewareDbError> {
    let query_and_params = QueryAndParams2 {
        query: query.to_string(),
        params,
    };

    match conn {
        MiddlewarePoolConnection::Sqlite(sqlite_conn) => {
            let result = sqlite_conn
                .interact(move |db_conn| {
                    let converted_params = convert_sql_params::<SqliteParamsQuery>(
                        &query_and_params.params,
                        ConversionMode::Query,
                    )?;
                    let tx = db_conn.transaction()?;

                    let result_set = {
                        let mut stmt = tx.prepare(&query_and_params.query)?;
                        
                        sql_middleware::sqlite_build_result_set(
                            &mut stmt,
                            &converted_params.0,
                        )?
                    };
                    tx.commit()?;
                    Ok::<_, SqlMiddlewareDbError>(result_set)
                })
                .await??;
                
            Ok(result)
        }
        _ => Err(SqlMiddlewareDbError::Other("Database type not supported for this operation".to_string())),
    }
}

pub async fn get_golfers_from_db(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<Vec<Scores>, SqlMiddlewareDbError> {
    // Helper functions for row extraction
    fn get_int(row: &sql_middleware::middleware::CustomDbRow, field: &str) -> i64 {
        row.get(field)
           .and_then(|v| v.as_int())
           .copied()
           .unwrap_or_default()
    }
    
    fn get_string(row: &sql_middleware::middleware::CustomDbRow, field: &str) -> String {
        row.get(field)
           .and_then(|v| v.as_text())
           .unwrap_or_default()
           .to_string()
    }

    let pool = config_and_pool.pool.get().await?;
    let conn = MiddlewarePool::get_connection(pool).await?;
    let query = match &conn {
        MiddlewarePoolConnection::Postgres(_) => {
            "SELECT grp, golfername, playername, eup_id, espn_id FROM sp_get_player_names($1) ORDER BY grp, eup_id"
        }
        MiddlewarePoolConnection::Sqlite(_) => {
            include_str!("admin/model/sql/functions/sqlite/02_sp_get_player_names.sql")
        }
        // &MiddlewarePoolConnection::Mssql(_) => todo!()
    };

    // Use the helper function to execute the query
    let query_result = execute_query(&conn, query, vec![RowValues2::Int(event_id as i64)]).await?;

    // Map the results to Scores objects
    let scores = query_result
        .results
        .iter()
        .map(|row| {
            Scores {
                group: get_int(row, "grp"),
                golfer_name: get_string(row, "golfername"),
                bettor_name: get_string(row, "bettorname"),
                eup_id: get_int(row, "eup_id"),
                espn_id: get_int(row, "espn_id"),
                detailed_statistics: Statistic {
                    eup_id: get_int(row, "eup_id"),
                    rounds: vec![],
                    round_scores: vec![],
                    tee_times: vec![],
                    holes_completed_by_round: vec![],
                    line_scores: vec![],
                    total_score: 0,
                },
            }
        })
        .collect();

    Ok(scores)
}

pub struct EventTitleAndScoreViewConf {
    pub event_name: String,
    pub score_view_step_factor: f32,
}

pub async fn get_title_and_score_view_conf_from_db(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<EventTitleAndScoreViewConf, SqlMiddlewareDbError> {
    let pool = config_and_pool.pool.get().await?;
    let conn = MiddlewarePool::get_connection(pool).await?;

    let query = match &conn {
        MiddlewarePoolConnection::Postgres(_) => {
            "SELECT EXISTS(SELECT 1 FROM event WHERE event_id = $1)"
        }
        MiddlewarePoolConnection::Sqlite(_) => {
            include_str!("admin/model/sql/functions/sqlite/01_sp_get_event_name.sql")
        }
        // &MiddlewarePoolConnection::Mssql(_) => todo!()
    };
    let query_and_params = QueryAndParams2 {
        query: query.to_string(),
        params: vec![RowValues2::Int(event_id as i64)],
    };

    let res = (match &conn {
        MiddlewarePoolConnection::Sqlite(sqlite_conn) => {
            sqlite_conn
                .interact(move |db_conn| {
                    let converted_params = convert_sql_params::<SqliteParamsQuery>(
                        &query_and_params.params,
                        ConversionMode::Query,
                    )?;
                    let tx = db_conn.transaction()?;

                    let result_set = {
                        let mut stmt = tx.prepare(&query_and_params.query)?;
                        
                        sql_middleware::sqlite_build_result_set(
                            &mut stmt,
                            &converted_params.0,
                        )?
                    };
                    tx.commit()?;
                    Ok::<_, SqlMiddlewareDbError>(result_set)
                })
                .await
        }
        _ => Ok(Err(SqlMiddlewareDbError::Other("Database type not supported for this operation".to_string()))),
    })?;

    let z = res?
        .results
        .iter()
        .map(|row| {
            Ok(EventTitleAndScoreViewConf {
                event_name: row
                    .get("eventname")
                    .and_then(|v| v.as_text())
                    .map(|v| v.to_string())
                    .ok_or(SqlMiddlewareDbError::Other("Name not found".to_string()))?,
                score_view_step_factor: row
                    .get("score_view_step_factor")
                    .and_then(|v| v.as_float())
                    .map(|v| v as f32)
                    .ok_or(SqlMiddlewareDbError::Other(
                        "Score view step factor not found".to_string(),
                    ))?,
            })
        })
        .last()
        .unwrap_or_else(|| Err(SqlMiddlewareDbError::Other("No results found".to_string())));

    z
}

pub async fn get_scores_from_db(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
    refresh_source: RefreshSource,
) -> Result<ScoresAndLastRefresh, SqlMiddlewareDbError> {
    let pool = config_and_pool.pool.get().await?;
    let conn = MiddlewarePool::get_connection(pool).await?;
    let query = match &conn {
        MiddlewarePoolConnection::Postgres(_) => {
            "SELECT grp, golfername, playername, eup_id, espn_id FROM sp_get_player_names($1) ORDER BY grp, eup_id"
        }
        MiddlewarePoolConnection::Sqlite(_) => {
            include_str!("admin/model/sql/functions/sqlite/03_sp_get_scores.sql")
        }
        // &MiddlewarePoolConnection::Mssql(_) => todo!()
    };
    let query_and_params = QueryAndParams2 {
        query: query.to_string(),
        params: vec![RowValues2::Int(event_id as i64)],
    };

    let res = (match &conn {
        MiddlewarePoolConnection::Sqlite(sqlite_conn) => {
            sqlite_conn
                .interact(move |db_conn| {
                    let converted_params = convert_sql_params::<SqliteParamsQuery>(
                        &query_and_params.params,
                        ConversionMode::Query,
                    )?;
                    let tx = db_conn.transaction()?;

                    let result_set = {
                        let mut stmt = tx.prepare(&query_and_params.query)?;
                        
                        sql_middleware::sqlite_build_result_set(
                            &mut stmt,
                            &converted_params.0,
                        )?
                    };
                    tx.commit()?;
                    Ok::<_, SqlMiddlewareDbError>(result_set)
                })
                .await
        }
        _ => Ok(Err(SqlMiddlewareDbError::Other("Database type not supported for this operation".to_string()))),
    })??;

    // Use a helper function to get the timestamp more clearly
    let last_time_updated = get_last_timestamp(&res.results);

    if cfg!(debug_assertions) {
        // let x = last_time_updated.format("%Y-%m-%d %H:%M:%S").to_string();
        // println!("model.rs ln 363 Last Time Updated: {:?}", x);
    }

    let z: Result<Vec<Scores>, SqlMiddlewareDbError> = res
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
                    rounds: parse_json_field(row, "rounds")?,
                    round_scores: parse_json_field(row, "round_scores")?,
                    tee_times: parse_json_field(row, "tee_times")?,
                    holes_completed_by_round: parse_json_field(row, "holes_completed_by_round")?,
                    line_scores: parse_json_field(row, "line_scores")?,
                    total_score: row
                        .get("total_score")
                        .and_then(|v| v.as_int())
                        .map(|v| *v as i32)
                        .unwrap_or_default(),
                },
            })
        })
        .collect::<Result<Vec<Scores>, SqlMiddlewareDbError>>();

    Ok(ScoresAndLastRefresh {
        score_struct: z?,
        last_refresh: last_time_updated,
        last_refresh_source: refresh_source,
    })
}

pub async fn store_scores_in_db(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
    scores: &[Scores],
) -> Result<(), SqlMiddlewareDbError> {
    fn build_insert_stmts(scores: &[Scores], event_id: i32) -> Result<Vec<QueryAndParams2>, SqlMiddlewareDbError> {
        let mut queries = vec![];
        for score in scores {
            let insert_stmt =
                include_str!("admin/model/sql/functions/sqlite/04_sp_set_eup_statistic.sql");
            
            // Convert all the JSON serialization to use proper error handling
            let rounds_json = serde_json::to_string(score.detailed_statistics.rounds.as_slice())
                .map_err(|e| SqlMiddlewareDbError::Other(format!("Failed to serialize rounds: {}", e)))?;
                
            let round_scores_json = serde_json::to_string(score.detailed_statistics.round_scores.as_slice())
                .map_err(|e| SqlMiddlewareDbError::Other(format!("Failed to serialize round scores: {}", e)))?;
                
            let tee_times_json = serde_json::to_string(score.detailed_statistics.tee_times.as_slice())
                .map_err(|e| SqlMiddlewareDbError::Other(format!("Failed to serialize tee times: {}", e)))?;
                
            let holes_completed_json = serde_json::to_string(
                score.detailed_statistics.holes_completed_by_round.as_slice())
                .map_err(|e| SqlMiddlewareDbError::Other(format!("Failed to serialize holes completed: {}", e)))?;
                
            let line_scores_json = serde_json::to_string(score.detailed_statistics.line_scores.as_slice())
                .map_err(|e| SqlMiddlewareDbError::Other(format!("Failed to serialize line scores: {}", e)))?;
            
            let param = vec![
                RowValues2::Int(event_id as i64),
                RowValues2::Int(score.espn_id),
                RowValues2::Int(score.eup_id),
                RowValues2::Int(score.group),
                RowValues2::Text(rounds_json),
                RowValues2::Text(round_scores_json),
                RowValues2::Text(tee_times_json),
                RowValues2::Text(holes_completed_json),
                RowValues2::Text(line_scores_json),
                RowValues2::Int(score.detailed_statistics.total_score as i64),
            ];
            queries.push(QueryAndParams2 {
                query: insert_stmt.to_string(),
                params: param,
            });
        }
        Ok(queries)
    }

    let pool = config_and_pool.pool.get().await?;
    let conn = MiddlewarePool::get_connection(pool).await?;
    let queries = build_insert_stmts(scores, event_id)?;

    if !queries.is_empty() {
        match &conn {
            MiddlewarePoolConnection::Sqlite(sqlite_conn) => {
                sqlite_conn
                    .interact(move |db_conn| {
                        let tx = db_conn.transaction()?;
                        {
                            // println!("Query: {:?}", queries[0].query);
                            let mut stmt = tx.prepare(&queries[0].query)?;

                            if cfg!(debug_assertions) {
                                let _x = stmt.expanded_sql();
                                // println!("Query from dbg: {:?}", x);
                            }
                            for query in queries {
                                let converted_params = convert_sql_params::<SqliteParamsExecute>(
                                    &query.params,
                                    ConversionMode::Execute,
                                )?;

                                let _rs = stmt.execute(converted_params.0)?;
                            }
                        }
                        tx.commit()?;
                        Ok::<_, SqlMiddlewareDbError>(())
                    })
                    .await??;
            }
            _ => {
                return Err(SqlMiddlewareDbError::Other("Database type not supported for this operation".to_string()))
            }
        }
    }
    Ok(())
}

pub async fn event_and_scores_already_in_db(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
    cache_max_age: i64,
) -> Result<bool, SqlMiddlewareDbError> {
    // Check if the event is set up in the database
    if get_title_and_score_view_conf_from_db(config_and_pool, event_id).await.is_err() {
        // If error, the event isn't setup, so we need to retrieve from ESPN
        return Ok(false);
    }

    let pool = config_and_pool.pool.get().await?;
    let conn = MiddlewarePool::get_connection(pool).await?;
    let query = match &conn {
        MiddlewarePoolConnection::Postgres(_) => {
            "SELECT min(ins_ts) as ins_ts FROM eup_statistic WHERE event_espn_id = $1;"
        }
        MiddlewarePoolConnection::Sqlite(_) => {
            include_str!(
                "admin/model/sql/functions/sqlite/05_sp_get_event_and_scores_already_in_db.sql"
            )
        }
        // &MiddlewarePoolConnection::Mssql(_) => todo!()
    };
    
    // Use the helper function to execute the query
    let query_result = execute_query(&conn, query, vec![RowValues2::Int(event_id as i64)]).await?;

    if let Some(results) = query_result.results.first() {
        let now = chrono::Utc::now().naive_utc();
        let val = results.get("ins_ts").and_then(|v| v.as_timestamp());
        if let Some(final_val) = val {
            if cfg!(debug_assertions) {
                #[allow(unused_variables)]
                let now_human_readable_fmt = now.format("%Y-%m-%d %H:%M:%S").to_string();
                let z_clone = final_val;
                #[allow(unused_variables)]
                let z_human_readable_fmt = z_clone.format("%Y-%m-%d %H:%M:%S").to_string();
                let diff = now.signed_duration_since(z_clone);
                #[allow(unused_variables)]
                let diff_days = diff.num_days();
                #[allow(unused_variables)]
                let pass = diff_days >= cache_max_age;
                println!(
                    "Now: {}, Last Refresh: {}, Diff: {} days, Pass: {}",
                    now_human_readable_fmt, z_human_readable_fmt, diff_days, pass
                );
            }

            let diff = now.signed_duration_since(final_val);
            Ok(diff.num_days() >= cache_max_age)
        } else {
            Ok(false)
        }
    } else {
        Ok(false)
    }
}
