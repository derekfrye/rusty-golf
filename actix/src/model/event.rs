use crate::model::execute_query;
use sql_middleware::SqlMiddlewareDbError;
use sql_middleware::middleware::RowValues as RowValues2;
use sql_middleware::middleware::{ConfigAndPool, MiddlewarePoolConnection};

pub struct EventTitleAndScoreViewConf {
    pub event_name: String,
    pub score_view_step_factor: f32,
    pub refresh_from_espn: i64,
    pub end_date: Option<String>,
}

/// # Errors
///
/// Will return `Err` if the database query fails
pub async fn get_event_details(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<EventTitleAndScoreViewConf, SqlMiddlewareDbError> {
    let mut conn = config_and_pool.get_connection().await?;

    let query = match &conn {
        MiddlewarePoolConnection::Postgres { .. } => {
            "SELECT name AS eventname, ins_ts, score_view_step_factor, refresh_from_espn, end_date \
             FROM event WHERE espn_id = $1"
        }
        MiddlewarePoolConnection::Sqlite { .. } => {
            include_str!("../sql/functions/sqlite/01_sp_get_event_details.sql")
        }
    };
    let params = vec![RowValues2::Int(i64::from(event_id))];
    let res = execute_query(&mut conn, query, params).await?;

    res.results
        .iter()
        .map(|row| {
            Ok(EventTitleAndScoreViewConf {
                event_name: row
                    .get("eventname")
                    .and_then(|v| v.as_text())
                    .map(ToString::to_string)
                    .ok_or(SqlMiddlewareDbError::Other("Name not found".to_string()))?,
                #[allow(clippy::cast_possible_truncation)]
                score_view_step_factor: row
                    .get("score_view_step_factor")
                    .and_then(sql_middleware::RowValues::as_float)
                    // Fallback to a sensible default if not set in DB
                    .map_or(3.0, |v| v as f32),
                refresh_from_espn: row
                    .get("refresh_from_espn")
                    .and_then(|v| v.as_int())
                    .copied()
                    .ok_or(SqlMiddlewareDbError::Other(
                        "Refresh from ESPN flag not found".to_string(),
                    ))?,
                end_date: row
                    .get("end_date")
                    .and_then(|v| v.as_text())
                    .map(ToString::to_string),
            })
        })
        .next_back()
        .unwrap_or(Err(SqlMiddlewareDbError::Other(
            "No results found".to_string(),
        )))
}
