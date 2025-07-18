use sql_middleware::middleware::{
    ConfigAndPool, ConversionMode, MiddlewarePool, MiddlewarePoolConnection,
};
use sql_middleware::{
    convert_sql_params, SqlMiddlewareDbError, SqliteParamsQuery,
};
use sql_middleware::middleware::{QueryAndParams as QueryAndParams2, RowValues as RowValues2};

pub struct EventTitleAndScoreViewConf {
    pub event_name: String,
    pub score_view_step_factor: f32,
    pub refresh_from_espn: i64,
}

pub async fn get_event_details(
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
            include_str!("../admin/model/sql/functions/sqlite/01_sp_get_event_details.sql")
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
    })?;

    res?
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
                refresh_from_espn: row
                    .get("refresh_from_espn")
                    .and_then(|v| v.as_int())
                    .copied()
                    .ok_or(SqlMiddlewareDbError::Other(
                        "Refresh from ESPN flag not found".to_string(),
                    ))?,
            })
        })
        .next_back()
        .unwrap_or_else(|| Err(SqlMiddlewareDbError::Other("No results found".to_string())))
}