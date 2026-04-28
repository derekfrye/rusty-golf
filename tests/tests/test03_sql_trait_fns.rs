mod common;
#[path = "support/test03.rs"]
mod test03;
#[path = "support/test03_build.rs"]
mod test03_build;

use rusty_golf_actix::controller::score::get_data_for_scores_page;
use rusty_golf_actix::model::ScoreData;
use rusty_golf_actix::storage::SqlStorage;
use serde_json::Value;
use sql_middleware::middleware::{ConfigAndPool as ConfigAndPool2, QueryAndParams, SqliteOptions};
use test03::run_miniflare_checks;

#[tokio::test]
async fn test3_sqlx_trait_get_scores() -> Result<(), Box<dyn std::error::Error>> {
    init_env();

    let storage = setup_sqlite_storage().await?;
    println!("Running SQL-backed assertions");
    let score_data = get_data_for_scores_page(401_580_351, 2024, false, &storage, 0)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let reference_result = reference_json()?;
    let expectations = assert_bryson_scores(&score_data, &reference_result);

    if run_serverless_enabled() {
        run_miniflare_checks(&storage, &expectations).await?;
    } else {
        println!("Skipping serverless checks: RUN_SERVERLESS=1 not set in .env");
    }

    Ok(())
}

pub(crate) struct BrysonExpectations {
    total_score: i32,
    line_score: i32,
    reference_eup_id: i64,
}

fn init_env() {
    let _ = dotenvy::dotenv();
    if std::env::var("MINIFLARE_URL").is_err() || std::env::var("MINIFLARE_ADMIN_TOKEN").is_err() {
        let _ = dotenvy::from_filename("../.env");
    }
}

fn run_serverless_enabled() -> bool {
    init_env();
    std::env::var("RUN_SERVERLESS").is_ok_and(|value| value.trim() == "1")
}

async fn setup_sqlite_storage() -> Result<SqlStorage, Box<dyn std::error::Error>> {
    let sqlite_options = SqliteOptions::new("file::memory:?cache=shared".to_string());
    let config_and_pool = ConfigAndPool2::new_sqlite(sqlite_options).await.unwrap();

    let ddl = [
        include_str!("../../actix/src/sql/schema/sqlite/00_event.sql"),
        // include_str!("../../actix/src/sql/schema/sqlite/01_golfstatistic.sql"),
        include_str!("../../actix/src/sql/schema/sqlite/02_golfer.sql"),
        include_str!("../../actix/src/sql/schema/sqlite/03_bettor.sql"),
        include_str!("../../actix/src/sql/schema/sqlite/04_event_user_player.sql"),
        include_str!("../../actix/src/sql/schema/sqlite/05_eup_statistic.sql"),
    ];

    let mut conn = config_and_pool.get_connection().await?;
    let query_and_params = QueryAndParams {
        query: ddl.join("\n"),
        params: vec![],
    };
    conn.execute_batch(&query_and_params.query).await?;

    let setup_queries = include_str!("test01.sql");
    let query_and_params = QueryAndParams {
        query: setup_queries.to_string(),
        params: vec![],
    };
    conn.execute_batch(&query_and_params.query).await?;

    Ok(SqlStorage::new(config_and_pool.clone()))
}

fn reference_json() -> Result<Value, Box<dyn std::error::Error>> {
    let reference_result_str = include_str!("test03_espn_json_responses.json");
    Ok(serde_json::from_str(reference_result_str)?)
}

fn assert_bryson_scores(score_data: &ScoreData, reference_result: &Value) -> BrysonExpectations {
    let bryson_espn_entry = score_data
        .score_struct
        .iter()
        .find(|s| s.golfer_name == "Bryson DeChambeau")
        .expect("Score data missing Bryson DeChambeau");
    let bryson_reference_entry = reference_result
        .get("score_struct")
        .and_then(Value::as_array)
        .and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry.get("golfer_name") == Some(&Value::from("Bryson DeChambeau")))
        })
        .expect("Reference JSON missing Bryson DeChambeau");

    let reference_total = bryson_reference_entry
        .get("detailed_statistics")
        .and_then(|stats| stats.get("total_score"))
        .and_then(Value::as_i64)
        .expect("Reference entry missing total_score");

    assert_eq!(
        i64::from(bryson_espn_entry.detailed_statistics.total_score),
        reference_total
    );

    let reference_eup_id = bryson_reference_entry
        .get("eup_id")
        .and_then(Value::as_i64)
        .expect("Reference entry missing eup_id");
    assert_eq!(bryson_espn_entry.eup_id, reference_eup_id);

    let reference_line_score = bryson_reference_entry
        .get("detailed_statistics")
        .and_then(|stats| stats.get("line_scores"))
        .and_then(Value::as_array)
        .and_then(|scores| {
            scores.iter().find(|s| {
                s.get("hole").and_then(Value::as_i64) == Some(13)
                    && s.get("round").and_then(Value::as_i64) == Some(2)
            })
        })
        .and_then(|entry| entry.get("score"))
        .and_then(Value::as_i64)
        .expect("Reference entry missing line score");
    let reference_line_score = i32::try_from(reference_line_score).expect("score fits in i32");

    let line_score = bryson_espn_entry
        .detailed_statistics
        .line_scores
        .iter()
        .find(|s| s.hole == 13 && s.round == 2)
        .expect("Score data missing line score")
        .score;
    assert_eq!(reference_line_score, line_score);
    assert_eq!(reference_line_score, 3); // line 6824 in test03_espn_json_responses.json

    BrysonExpectations {
        total_score: bryson_espn_entry.detailed_statistics.total_score,
        line_score,
        reference_eup_id,
    }
}
