use sql_middleware::middleware::{ConfigAndPool, MiddlewarePoolConnection};
use sql_middleware::middleware::{QueryAndParams as QueryAndParams2, RowValues as RowValues2};
use sql_middleware::SqlMiddlewareDbError;

use crate::model::types::Scores;

/// # Errors
///
/// Will return `Err` if the database query fails
pub async fn execute_batch_sql(
    config_and_pool: &ConfigAndPool,
    query: &str,
) -> Result<(), SqlMiddlewareDbError> {
    let mut conn = config_and_pool.get_connection().await?;

    conn.execute_batch(query).await
}

/// # Errors
///
/// Will return `Err` if the database query fails
pub async fn store_scores_in_db(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
    scores: &[Scores],
) -> Result<(), SqlMiddlewareDbError> {
    let mut conn = config_and_pool.get_connection().await?;
    let queries = build_insert_queries(scores, event_id)?;

    if queries.is_empty() {
        return Ok(());
    }

    match &mut conn {
        sqlite_conn @ MiddlewarePoolConnection::Sqlite { .. } => {
            execute_sqlite_queries(sqlite_conn, queries).await?;
        }
        MiddlewarePoolConnection::Postgres { .. } => {
            return Err(SqlMiddlewareDbError::Other(
                "Database type not supported for this operation".to_string(),
            ));
        }
    }
    Ok(())
}

fn build_insert_queries(
    scores: &[Scores],
    event_id: i32,
) -> Result<Vec<QueryAndParams2>, SqlMiddlewareDbError> {
    let mut queries = vec![];
    for score in scores {
        let insert_stmt = include_str!("../sql/functions/sqlite/04_sp_set_eup_statistic.sql");

        let rounds_json = serde_json::to_string(&score.detailed_statistics.rounds)
            .map_err(|e| SqlMiddlewareDbError::Other(format!("Failed to serialize rounds: {e}")))?;

        let round_scores_json = serde_json::to_string(&score.detailed_statistics.round_scores)
            .map_err(|e| {
                SqlMiddlewareDbError::Other(format!("Failed to serialize round scores: {e}"))
            })?;

        let tee_times_json =
            serde_json::to_string(&score.detailed_statistics.tee_times).map_err(|e| {
                SqlMiddlewareDbError::Other(format!("Failed to serialize tee times: {e}"))
            })?;

        let holes_completed_json = serde_json::to_string(
            &score.detailed_statistics.holes_completed_by_round,
        )
        .map_err(|e| {
            SqlMiddlewareDbError::Other(format!("Failed to serialize holes completed: {e}"))
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

async fn execute_sqlite_queries(
    sqlite_conn: &mut MiddlewarePoolConnection,
    queries: Vec<QueryAndParams2>,
) -> Result<(), SqlMiddlewareDbError> {
    let Some(first) = queries.first() else {
        return Ok(());
    };

    let insert_sql = first.query.clone();
    let mut prepared = sqlite_conn
        .prepare_sqlite_statement(&insert_sql)
        .await?;

    for query in queries {
        prepared.execute(&query.params).await?;
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

    let mut conn = config_and_pool.get_connection().await?;
    let query = match &conn {
        MiddlewarePoolConnection::Postgres { .. } => {
            "SELECT min(ins_ts) as ins_ts FROM eup_statistic WHERE event_espn_id = $1;"
        }
        MiddlewarePoolConnection::Sqlite { .. } => {
            include_str!("../sql/functions/sqlite/05_sp_get_event_and_scores_already_in_db.sql")
        }
    };

    let query_result =
        execute_query(&mut conn, query, vec![RowValues2::Int(i64::from(event_id))]).await?;

    if let Some(results) = query_result.results.first() {
        let now = chrono::Utc::now().naive_utc();
        let val = results
            .get("ins_ts")
            .and_then(sql_middleware::RowValues::as_timestamp);
        if let Some(final_val) = val {
            if cfg!(debug_assertions) {
                let now_human_readable_fmt = now.format("%Y-%m-%d %H:%M:%S").to_string();
                let z_clone = final_val;
                let z_human_readable_fmt = z_clone.format("%Y-%m-%d %H:%M:%S").to_string();
                let diff = now.signed_duration_since(z_clone);
                let diff_days = diff.num_days();
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
