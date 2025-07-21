use sql_middleware::middleware::{
    ConfigAndPool, ConversionMode, MiddlewarePool, MiddlewarePoolConnection,
};
use sql_middleware::middleware::{QueryAndParams as QueryAndParams2, RowValues as RowValues2};
use sql_middleware::{SqlMiddlewareDbError, SqliteParamsExecute, convert_sql_params};

use crate::model::types::Scores;

/// # Errors
///
/// Will return `Err` if the database query fails
pub async fn execute_batch_sql(
    config_and_pool: &ConfigAndPool,
    query: &str,
) -> Result<(), SqlMiddlewareDbError> {
    let pool = config_and_pool.pool.get().await?;
    let sconn = MiddlewarePool::get_connection(pool).await?;
    let query_and_params = QueryAndParams2 {
        query: query.to_string(),
        params: vec![],
    };

    match sconn {
        MiddlewarePoolConnection::Postgres(mut xx) => {
            let tx = xx.transaction().await?;
            tx.batch_execute(&query_and_params.query).await?;
            tx.commit().await?;
            Ok::<_, SqlMiddlewareDbError>(())
        }
        MiddlewarePoolConnection::Sqlite(xx) => {
            xx.interact(move |xxx| {
                let tx = xxx.transaction()?;
                tx.execute_batch(&query_and_params.query)?;
                tx.commit()?;
                Ok::<_, SqlMiddlewareDbError>(())
            })
            .await??;
            Ok(())
        }
    }
}

/// # Errors
///
/// Will return `Err` if the database query fails
pub async fn store_scores_in_db(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
    scores: &[Scores],
) -> Result<(), SqlMiddlewareDbError> {
    fn build_insert_stmts(
        scores: &[Scores],
        event_id: i32,
    ) -> Result<Vec<QueryAndParams2>, SqlMiddlewareDbError> {
        let mut queries = vec![];
        for score in scores {
            let insert_stmt =
                include_str!("../admin/model/sql/functions/sqlite/04_sp_set_eup_statistic.sql");

            let rounds_json =
                serde_json::to_string(&score.detailed_statistics.rounds).map_err(|e| {
                    SqlMiddlewareDbError::Other(format!("Failed to serialize rounds: {e}"))
                })?;

            let round_scores_json = serde_json::to_string(&score.detailed_statistics.round_scores)
                .map_err(|e| {
                    SqlMiddlewareDbError::Other(format!("Failed to serialize round scores: {e}"))
                })?;

            let tee_times_json = serde_json::to_string(&score.detailed_statistics.tee_times)
                .map_err(|e| {
                    SqlMiddlewareDbError::Other(format!("Failed to serialize tee times: {e}"))
                })?;

            let holes_completed_json =
                serde_json::to_string(&score.detailed_statistics.holes_completed_by_round)
                    .map_err(|e| {
                        SqlMiddlewareDbError::Other(format!(
                            "Failed to serialize holes completed: {e}"
                        ))
                    })?;

            let line_scores_json = serde_json::to_string(&score.detailed_statistics.line_scores)
                .map_err(|e| {
                    SqlMiddlewareDbError::Other(format!("Failed to serialize line scores: {e}"))
                })?;

            let param = vec![
                RowValues2::Int(i64::from(event_id)),
                RowValues2::Int(score.espn_id),
                RowValues2::Int(score.eup_id),
                RowValues2::Int(score.group),
                RowValues2::Text(rounds_json),
                RowValues2::Text(round_scores_json),
                RowValues2::Text(tee_times_json),
                RowValues2::Text(holes_completed_json),
                RowValues2::Text(line_scores_json),
                RowValues2::Int(i64::from(score.detailed_statistics.total_score)),
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
                            let mut stmt = tx.prepare(&queries[0].query)?;

                            if cfg!(debug_assertions) {
                                let _x = stmt.expanded_sql();
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
            MiddlewarePoolConnection::Postgres(_) => {
                return Err(SqlMiddlewareDbError::Other(
                    "Database type not supported for this operation".to_string(),
                ));
            }
        }
    }
    Ok(())
}

/// # Errors
///
/// Will return `Err` if the database query fails
pub async fn event_and_scores_already_in_db(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
    cache_max_age: i64,
) -> Result<bool, SqlMiddlewareDbError> {
    use crate::model::database_read::execute_query;
    use crate::model::event::get_event_details;
    use sql_middleware::middleware::RowValues as RowValues2;

    if get_event_details(config_and_pool, event_id).await.is_err() {
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
                "../admin/model/sql/functions/sqlite/05_sp_get_event_and_scores_already_in_db.sql"
            )
        }
    };

    let query_result =
        execute_query(&conn, query, vec![RowValues2::Int(i64::from(event_id))]).await?;

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
                    "Now: {now_human_readable_fmt}, Last Refresh: {z_human_readable_fmt}, Diff: {diff_days} days, Pass: {pass}"
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
