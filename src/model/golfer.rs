use sql_middleware::SqlMiddlewareDbError;
use sql_middleware::middleware::RowValues as RowValues2;
use sql_middleware::middleware::{ConfigAndPool, MiddlewarePool, MiddlewarePoolConnection};
use std::collections::HashMap;

use crate::model::database_read::execute_query;
use crate::model::score::Statistic;
use crate::model::types::Scores;

/// # Errors
///
/// Will return `Err` if the database query fails
pub async fn get_golfers_from_db(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<Vec<Scores>, SqlMiddlewareDbError> {
    fn get_int(row: &sql_middleware::middleware::CustomDbRow, field: &str) -> i64 {
        row.get(field).and_then(|v| v.as_int()).map_or(0, |&v| v)
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
            include_str!("../admin/model/sql/functions/sqlite/02_sp_get_player_names.sql")
        }
    };

    let query_result =
        execute_query(&conn, query, vec![RowValues2::Int(i64::from(event_id))]).await?;

    let scores = query_result
        .results
        .iter()
        .map(|row| Scores {
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
            score_view_step_factor: None,
        })
        .collect();

    Ok(scores)
}

/// # Errors
///
/// Will return `Err` if the database query fails
pub async fn get_player_step_factors(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<HashMap<(i64, String), f32>, SqlMiddlewareDbError> {
    let pool = config_and_pool.pool.get().await?;
    let conn = MiddlewarePool::get_connection(pool).await?;

    let query = include_str!("../admin/model/sql/functions/sqlite/03_sp_get_scores.sql");

    let query_result =
        execute_query(&conn, query, vec![RowValues2::Int(i64::from(event_id))]).await?;

    let step_factors: HashMap<(i64, String), f32> = query_result
        .results
        .iter()
        .filter_map(|row| {
            let golfer_espn_id = row
                .get("golfer_espn_id")
                .and_then(|v| v.as_int())
                .copied()?;

            let bettor_name = row.get("bettorname").and_then(|v| v.as_text())?.to_string();

            let step_factor = row
                .get("score_view_step_factor")
                .and_then(|v| v.as_float())
                .map(|v| v as f32)?;

            Some(((golfer_espn_id, bettor_name), step_factor))
        })
        .collect();

    Ok(step_factors)
}
