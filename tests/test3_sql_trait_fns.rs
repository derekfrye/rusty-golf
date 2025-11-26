use serde_json::Value;
// use sqlx::sqlite::SqlitePoolOptions;
use std::vec;

// use rusty_golf::controller::score;
use rusty_golf::controller::score::get_data_for_scores_page;

use sql_middleware::middleware::{
    ConfigAndPool as ConfigAndPool2, QueryAndParams,
};

#[tokio::test]
async fn test3_sqlx_trait_get_scores() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (optional, but useful for debugging)
    // let _ = env_logger::builder().is_test(true).try_init();

    let x = "file::memory:?cache=shared".to_string();
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

    let setup_queries = include_str!("test1.sql");
    let query_and_params = QueryAndParams {
        query: setup_queries.to_string(),
        params: vec![],
    };

    conn.execute_batch(&query_and_params.query).await?;

    let x = match get_data_for_scores_page(401580351, 2024, false, &config_and_pool, 0).await {
        Ok(data) => data,
        Err(e) => return Err(e),
    };

    // load reference json
    let reference_result_str = include_str!("test3_espn_json_responses.json");
    let reference_result: Value = serde_json::from_str(reference_result_str).unwrap();

    let bryson_espn_entry = x
        .score_struct
        .iter()
        .find(|s| s.golfer_name == "Bryson DeChambeau")
        .unwrap();
    let bryson_reference_entry = reference_result
        .get("score_struct")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s.get("golfer_name").unwrap() == "Bryson DeChambeau")
        .unwrap();
    assert_eq!(
        bryson_espn_entry.detailed_statistics.total_score as i64,
        bryson_reference_entry
            .get("detailed_statistics")
            .unwrap()
            .get("total_score")
            .unwrap()
            .as_i64()
            .unwrap()
    );
    assert_eq!(
        bryson_espn_entry.eup_id,
        bryson_reference_entry
            .get("eup_id")
            .unwrap()
            .as_i64()
            .unwrap()
    );

    // let xx = serde_json::to_string_pretty(&x).unwrap();
    // println!("{}", xx); // test3_scoredata.json
    // let a = 1 + 1;

    let left = bryson_reference_entry
        .get("detailed_statistics")
        .unwrap()
        .get("line_scores")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|s| {
            s.get("hole").unwrap().as_i64().unwrap() == 13
                && s.get("round").unwrap().as_i64().unwrap() == 2
        })
        .unwrap()
        .get("score")
        .unwrap()
        .as_i64()
        .unwrap() as i32;
    println!("{left}");
    let right = bryson_espn_entry
        .detailed_statistics
        .line_scores
        .iter()
        .find(|s| s.hole == 13 && s.round == 2)
        .unwrap()
        .score;
    println!("{right}");

    assert_eq!(left, right);
    assert_eq!(left, 3); // line 6824 in test3_espn_json_responses.json

    Ok(())
}
