use serde_json::Value;
// use sqlx::sqlite::SqlitePoolOptions;
// use rusty_golf_actix::controller::score;
use rusty_golf_actix::controller::score::get_data_for_scores_page;
use rusty_golf_actix::model::format_time_ago_for_score_view;
use rusty_golf_actix::model::{Bettors, ScoreData, ScoresAndLastRefresh};
use rusty_golf_actix::storage::{R2Storage, SqlStorage};
use rusty_golf_actix::view::score::{
    render_scores_template_pure, scores_and_last_refresh_to_line_score_tables,
};
use rusty_golf_core::storage::Storage;

use sql_middleware::middleware::{ConfigAndPool as ConfigAndPool2, QueryAndParams, SqliteOptions};

#[tokio::test]
async fn test3_sqlx_trait_get_scores() -> Result<(), Box<dyn std::error::Error>> {
    init_env();
    if r2_ready() {
        println!("R2 test portions will run");
    }

    let storage = setup_sqlite_storage().await?;
    println!("Running SQL-backed assertions");
    let score_data = get_data_for_scores_page(401_580_351, 2024, false, &storage, 0)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let reference_result = reference_json()?;
    let expectations = assert_bryson_scores(&score_data, &reference_result);

    if r2_ready() {
        run_r2_checks(&score_data, &storage, &expectations).await?;
    } else {
        println!("Skipping R2 checks: missing R2 env vars");
    }

    Ok(())
}

struct BrysonExpectations {
    total_score: i32,
    line_score: i32,
    reference_eup_id: i64,
}

fn init_env() {
    let _ = dotenvy::dotenv();
    if std::env::var("R2_ENDPOINT").is_err() {
        let _ = dotenvy::from_filename("../.env");
    }
}

fn r2_ready() -> bool {
    std::env::var("R2_ENDPOINT").is_ok()
        && std::env::var("R2_BUCKET").is_ok()
        && (std::env::var("R2_ACCESS_KEY_ID").is_ok() || std::env::var("AWS_ACCESS_KEY_ID").is_ok())
        && (std::env::var("R2_SECRET_ACCESS_KEY").is_ok()
            || std::env::var("AWS_SECRET_ACCESS_KEY").is_ok())
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

async fn run_r2_checks(
    score_data: &ScoreData,
    storage: &SqlStorage,
    expectations: &BrysonExpectations,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running R2-backed assertions");
    let r2_config = R2Storage::config_from_env()?;
    let signer = R2Storage::signer_from_config(&r2_config);
    let r2_storage = R2Storage::new(r2_config, std::sync::Arc::new(signer));

    r2_storage
        .store_scores(401_580_351, &score_data.score_struct)
        .await?;
    let r2_scores = r2_storage
        .get_scores(401_580_351, rusty_golf_actix::model::RefreshSource::Db)
        .await?;

    assert_eq!(
        score_data.score_struct.len(),
        r2_scores.score_struct.len(),
        "R2 score count mismatch"
    );

    let r2_bryson = r2_scores
        .score_struct
        .iter()
        .find(|s| s.golfer_name == "Bryson DeChambeau")
        .expect("R2 scores missing Bryson DeChambeau");
    assert_eq!(
        expectations.total_score, r2_bryson.detailed_statistics.total_score,
        "R2 total score mismatch for Bryson DeChambeau"
    );
    assert_eq!(
        expectations.reference_eup_id, r2_bryson.eup_id,
        "R2 eup_id mismatch for Bryson DeChambeau"
    );
    let r2_line_score = r2_bryson
        .detailed_statistics
        .line_scores
        .iter()
        .find(|s| s.hole == 13 && s.round == 2)
        .expect("R2 line score missing")
        .score;
    assert_eq!(
        expectations.line_score, r2_line_score,
        "R2 line score mismatch"
    );

    let r2_data = build_score_data_from_scores(&r2_scores);
    let from_db_scores = storage
        .get_scores(401_580_351, rusty_golf_actix::model::RefreshSource::Db)
        .await?;
    let bettor_struct = scores_and_last_refresh_to_line_score_tables(&from_db_scores);
    let event_details = storage.get_event_details(401_580_351).await?;
    let player_step_factors = storage.get_player_step_factors(401_580_351).await?;

    let markup = render_scores_template_pure(
        &r2_data,
        false,
        &bettor_struct,
        event_details.score_view_step_factor,
        &player_step_factors,
        401_580_351,
        2024,
        true,
    );

    assert!(
        !markup.into_string().is_empty(),
        "R2-rendered markup should not be empty"
    );
    Ok(())
}

fn build_score_data_from_scores(scores: &ScoresAndLastRefresh) -> ScoreData {
    use std::collections::HashMap;

    let mut totals: HashMap<String, i32> = HashMap::new();
    for golfer in &scores.score_struct {
        *totals.entry(golfer.bettor_name.clone()).or_insert(0) +=
            golfer.detailed_statistics.total_score;
    }

    let mut bettors: Vec<Bettors> = totals
        .into_iter()
        .map(|(name, total)| Bettors {
            bettor_name: name,
            total_score: total,
            scoreboard_position_name: String::new(),
            scoreboard_position: 0,
        })
        .collect();

    bettors.sort_by(|a, b| {
        a.total_score
            .cmp(&b.total_score)
            .then_with(|| a.bettor_name.cmp(&b.bettor_name))
    });

    for (i, bettor) in bettors.iter_mut().enumerate() {
        bettor.scoreboard_position = i;
        bettor.scoreboard_position_name = match i {
            0 => "TOP GOLFER".to_string(),
            1 => "FIRST LOSER".to_string(),
            2 => "MEH".to_string(),
            3 => "SEEN BETTER DAYS".to_string(),
            4 => "NOT A CHANCE".to_string(),
            _ => "WORST OF THE WORST".to_string(),
        };
    }

    let x = chrono::Utc::now().naive_utc() - scores.last_refresh;

    ScoreData {
        bettor_struct: bettors,
        score_struct: scores.score_struct.clone(),
        last_refresh: format_time_ago_for_score_view(x),
        last_refresh_source: scores.last_refresh_source.clone(),
    }
}
