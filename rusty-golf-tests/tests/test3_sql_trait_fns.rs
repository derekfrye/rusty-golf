use serde_json::Value;
// use sqlx::sqlite::SqlitePoolOptions;
use std::vec;

// use rusty_golf::controller::score;
use rusty_golf::controller::score::get_data_for_scores_page;
use rusty_golf::model::{Bettors, ScoreData, ScoresAndLastRefresh};
use rusty_golf::model::format_time_ago_for_score_view;
use rusty_golf::storage::{R2Storage, SqlStorage};
use rusty_golf::view::score::{
    render_scores_template_pure, scores_and_last_refresh_to_line_score_tables,
};
use rusty_golf_core::storage::Storage;

use sql_middleware::middleware::{
    ConfigAndPool as ConfigAndPool2, QueryAndParams, SqliteOptions,
};

#[tokio::test]
async fn test3_sqlx_trait_get_scores() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (optional, but useful for debugging)
    // let _ = env_logger::builder().is_test(true).try_init();
    let _ = dotenvy::dotenv();
    if std::env::var("R2_ENDPOINT").is_err() {
        let _ = dotenvy::from_filename("../.env");
    }

    if std::env::var("R2_ENDPOINT").is_ok() {
        println!("R2 test portions will run");
    }

    let x = "file::memory:?cache=shared".to_string();
    let sqlite_options = SqliteOptions::new(x.clone());
    let config_and_pool = ConfigAndPool2::new_sqlite(sqlite_options).await.unwrap();

    let ddl = [
        include_str!("../../rusty-golf-actix/src/sql/schema/sqlite/00_event.sql"),
        // include_str!("../../rusty-golf-actix/src/sql/schema/sqlite/01_golfstatistic.sql"),
        include_str!("../../rusty-golf-actix/src/sql/schema/sqlite/02_golfer.sql"),
        include_str!("../../rusty-golf-actix/src/sql/schema/sqlite/03_bettor.sql"),
        include_str!("../../rusty-golf-actix/src/sql/schema/sqlite/04_event_user_player.sql"),
        include_str!("../../rusty-golf-actix/src/sql/schema/sqlite/05_eup_statistic.sql"),
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

    let storage = SqlStorage::new(config_and_pool.clone());

    println!("Running SQL-backed assertions");
    let x = match get_data_for_scores_page(401_580_351, 2024, false, &storage, 0).await {
        Ok(data) => data,
        Err(e) => return Err(Box::new(e) as Box<dyn std::error::Error>),
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
        i64::from(bryson_espn_entry.detailed_statistics.total_score),
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

    let left = i32::try_from(
        bryson_reference_entry
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
            .unwrap(),
    )
    .expect("score fits in i32");
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

    let r2_ready = std::env::var("R2_ENDPOINT").is_ok()
        && std::env::var("R2_BUCKET").is_ok()
        && (std::env::var("R2_ACCESS_KEY_ID").is_ok() || std::env::var("AWS_ACCESS_KEY_ID").is_ok())
        && (std::env::var("R2_SECRET_ACCESS_KEY").is_ok()
            || std::env::var("AWS_SECRET_ACCESS_KEY").is_ok());

    if r2_ready {
        println!("Running R2-backed assertions");
        let r2_config = R2Storage::config_from_env()?;
        let signer = R2Storage::signer_from_config(&r2_config);
        let r2_storage = R2Storage::new(r2_config, std::sync::Arc::new(signer));

        r2_storage.store_scores(401_580_351, &x.score_struct).await?;
        let r2_scores = r2_storage
            .get_scores(401_580_351, rusty_golf::model::RefreshSource::Db)
            .await?;

        assert_eq!(
            x.score_struct.len(),
            r2_scores.score_struct.len(),
            "R2 score count mismatch"
        );

        let r2_bryson = r2_scores
            .score_struct
            .iter()
            .find(|s| s.golfer_name == "Bryson DeChambeau")
            .expect("R2 scores missing Bryson DeChambeau");
        assert_eq!(
            bryson_espn_entry.detailed_statistics.total_score,
            r2_bryson.detailed_statistics.total_score,
            "R2 total score mismatch for Bryson DeChambeau"
        );
        assert_eq!(
            bryson_reference_entry
                .get("eup_id")
                .unwrap()
                .as_i64()
                .unwrap(),
            r2_bryson.eup_id,
            "R2 eup_id mismatch for Bryson DeChambeau"
        );
        let r2_line_score = r2_bryson
            .detailed_statistics
            .line_scores
            .iter()
            .find(|s| s.hole == 13 && s.round == 2)
            .unwrap()
            .score;
        assert_eq!(left, r2_line_score, "R2 line score mismatch");

        let r2_data = build_score_data_from_scores(&r2_scores);
        let from_db_scores = storage
            .get_scores(401_580_351, rusty_golf::model::RefreshSource::Db)
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
    } else {
        println!("Skipping R2 checks: missing R2 env vars");
    }

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
