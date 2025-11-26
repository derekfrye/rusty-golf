mod common;
use crate::common::ConnExt;
use std::fs;
use std::path::PathBuf;
// use sqlx::sqlite::SqlitePoolOptions;
use std::vec;

// use rusty_golf::controller::score;
use rusty_golf::controller::score::get_data_for_scores_page;

use sql_middleware::middleware::{ConfigAndPool as ConfigAndPool2, QueryAndParams, RowValues};

#[tokio::test]
async fn test4_get_scores_from_cache() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (optional, but useful for debugging)
    // let _ = env_logger::builder().is_test(true).try_init();

    let x = "file::memory:?cache=shared".to_string();

    let now = chrono::Utc::now().naive_utc();
    let eleven_days_ago = now.checked_add_signed(chrono::Duration::days(-11)).unwrap();
    // let time_delta = now - eleven_days_ago;

    let database_exists = {
        let path = PathBuf::from(&x);
        if !path.is_file() || fs::metadata(&path).is_err() {
            false
        } else {
            // we're going to update the timestamp in the ins_ts column
            let query = "update eup_statistic set ins_ts = ?1;";
            // let now = chrono::Utc::now().naive_utc();

            let eleven_days_ago_h = eleven_days_ago.format("%Y-%m-%d %H:%M:%S").to_string();
            let params = vec![RowValues::Text(eleven_days_ago_h)];

            let config_and_pool = ConfigAndPool2::new_sqlite(x.clone()).await.unwrap();
            let mut conn = config_and_pool.get_connection().await?;

            conn.execute_dml(query, &params).await?;
            true
        }
    };
    let config_and_pool = ConfigAndPool2::new_sqlite(x.clone()).await.unwrap();

    let ddl = [
        include_str!("../src/sql/schema/sqlite/00_event.sql"),
        // include_str!("../src/sql/schema/sqlite/01_golfstatistic.sql"),
        include_str!("../src/sql/schema/sqlite/02_golfer.sql"),
        include_str!("../src/sql/schema/sqlite/03_bettor.sql"),
        include_str!("../src/sql/schema/sqlite/04_event_user_player.sql"),
        include_str!("../src/sql/schema/sqlite/05_eup_statistic.sql"),
    ];

    let query_and_params = QueryAndParams {
        query: ddl.join("\n"),
        params: vec![],
    };

    let mut conn = config_and_pool.get_connection().await?;

    conn.execute_batch(&query_and_params.query).await?;

    if !database_exists {
        let setup_queries = include_str!("test1.sql");
        let query_and_params = QueryAndParams {
            query: setup_queries.to_string(),
            params: vec![],
        };

        conn.execute_batch(&query_and_params.query).await?;
    }

    let score_data = if database_exists {
        // cache max age of 0 means always use db, since model.event_and_scores_already_in_db does this:
        // let now = chrono::Utc::now().naive_utc();
        // let diff = now - z?;
        // Ok(diff.num_days() > cache_max_age)
        get_data_for_scores_page(401_580_351, 2024, true, &config_and_pool, 0).await
    } else {
        if cfg!(debug_assertions) {
            println!("db didn't exist, set data back 11 days");
        }
        get_data_for_scores_page(401_580_351, 2024, false, &config_and_pool, 99).await?;
        // now set the data back 11 days
        let query = "update eup_statistic set ins_ts = ?1;";

        let eleven_days_ago_h = eleven_days_ago.format("%Y-%m-%d %H:%M:%S").to_string();
        let params = vec![RowValues::Text(eleven_days_ago_h.clone())];

        let config_and_pool = ConfigAndPool2::new_sqlite(x.clone()).await.unwrap();
        let mut conn = config_and_pool.get_connection().await?;

        conn.execute_dml(query, &params).await?;
        get_data_for_scores_page(401_580_351, 2024, true, &config_and_pool, 0).await
    }?;

    if cfg!(debug_assertions) {
        println!("last_refresh: {}", score_data.last_refresh);
    }

    // now linked to format_time_ago_for_score_view()
    assert_eq!(score_data.last_refresh, "1 week");

    Ok(())
}
