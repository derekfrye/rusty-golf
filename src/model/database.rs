use chrono::NaiveDateTime;
use sql_middleware::middleware::{
    ConfigAndPool, ConversionMode, MiddlewarePool, MiddlewarePoolConnection, ResultSet,
};
use sql_middleware::{
    convert_sql_params, SqlMiddlewareDbError, SqliteParamsExecute, SqliteParamsQuery,
};
use sql_middleware::middleware::{QueryAndParams as QueryAndParams2, RowValues as RowValues2};

use crate::model::types::{RefreshSource, Scores, ScoresAndLastRefresh};
use crate::model::score::Statistic;

pub fn parse_json_field<T>(
    row: &sql_middleware::middleware::CustomDbRow,
    field_name: &str,
) -> Result<T, SqlMiddlewareDbError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let json_text = row
        .get(field_name)
        .and_then(|v| v.as_text())
        .unwrap_or_default();

    serde_json::from_str(json_text).map_err(|e| {
        SqlMiddlewareDbError::Other(format!("Failed to parse {} field: {}", field_name, e))
    })
}

pub fn get_last_timestamp(
    results: &[sql_middleware::middleware::CustomDbRow],
) -> chrono::NaiveDateTime {
    results
        .iter()
        .filter_map(|row| row.get("ins_ts").and_then(|v| v.as_timestamp()))
        .next_back()
        .unwrap_or_else(|| chrono::Utc::now().naive_utc())
}

pub async fn execute_query(
    conn: &MiddlewarePoolConnection,
    query: &str,
    params: Vec<RowValues2>,
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

                        sql_middleware::sqlite_build_result_set(&mut stmt, &converted_params.0)?
                    };
                    tx.commit()?;
                    Ok::<_, SqlMiddlewareDbError>(result_set)
                })
                .await??;

            Ok(result)
        }
        _ => Err(SqlMiddlewareDbError::Other(
            "Database type not supported for this operation".to_string(),
        )),
    }
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
            include_str!("../admin/model/sql/functions/sqlite/03_sp_get_scores.sql")
        }
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

                        sql_middleware::sqlite_build_result_set(&mut stmt, &converted_params.0)?
                    };
                    tx.commit()?;
                    Ok::<_, SqlMiddlewareDbError>(result_set)
                })
                .await
        }
        _ => Ok(Err(SqlMiddlewareDbError::Other(
            "Database type not supported for this operation".to_string(),
        ))),
    })??;

    let last_time_updated = get_last_timestamp(&res.results);

    if cfg!(debug_assertions) {
        // Debug logging would go here
    }

    let z: Result<Vec<Scores>, SqlMiddlewareDbError> = res
        .results
        .iter()
        .map(|row| {
            Ok(Scores {
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
                score_view_step_factor: row
                    .get("score_view_step_factor")
                    .and_then(|v| v.as_float())
                    .map(|v| v as f32),
            })
        })
        .collect::<Result<Vec<Scores>, SqlMiddlewareDbError>>();

    Ok(ScoresAndLastRefresh {
        score_struct: z?,
        last_refresh: last_time_updated,
        last_refresh_source: refresh_source,
    })
}