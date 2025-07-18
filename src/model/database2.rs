use sql_middleware::middleware::{
    ConfigAndPool, ConversionMode, MiddlewarePool, MiddlewarePoolConnection,
};
use sql_middleware::{
    convert_sql_params, SqlMiddlewareDbError, SqliteParamsExecute,
};
use sql_middleware::middleware::{QueryAndParams as QueryAndParams2, RowValues as RowValues2};

use crate::model::types::Scores;

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

            let rounds_json = serde_json::to_string(score.detailed_statistics.rounds.as_slice())
                .map_err(|e| {
                    SqlMiddlewareDbError::Other(format!("Failed to serialize rounds: {}", e))
                })?;

            let round_scores_json = serde_json::to_string(
                score.detailed_statistics.round_scores.as_slice(),
            )
            .map_err(|e| {
                SqlMiddlewareDbError::Other(format!("Failed to serialize round scores: {}", e))
            })?;

            let tee_times_json = serde_json::to_string(
                score.detailed_statistics.tee_times.as_slice(),
            )
            .map_err(|e| {
                SqlMiddlewareDbError::Other(format!("Failed to serialize tee times: {}", e))
            })?;

            let holes_completed_json = serde_json::to_string(
                score
                    .detailed_statistics
                    .holes_completed_by_round
                    .as_slice(),
            )
            .map_err(|e| {
                SqlMiddlewareDbError::Other(format!("Failed to serialize holes completed: {}", e))
            })?;

            let line_scores_json = serde_json::to_string(
                score.detailed_statistics.line_scores.as_slice(),
            )
            .map_err(|e| {
                SqlMiddlewareDbError::Other(format!("Failed to serialize line scores: {}", e))
            })?;

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
            _ => {
                return Err(SqlMiddlewareDbError::Other(
                    "Database type not supported for this operation".to_string(),
                ));
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
    use crate::model::event::get_event_details;
    use crate::model::database::execute_query;
    use sql_middleware::middleware::RowValues as RowValues2;

    if get_event_details(config_and_pool, event_id)
        .await
        .is_err()
    {
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