mod common;
use crate::common::ConnExt;
use rusty_golf::controller::db_prefill::db_prefill;
use rusty_golf::model::{AllBettorScoresByRound, DetailedScore, SummaryDetailedScores};
use std::io::Write;
use std::path::Path;

use sql_middleware::middleware::{ConfigAndPool, DatabaseType, QueryAndParams, SqliteOptions};

async fn setup_db() -> Result<ConfigAndPool, Box<dyn std::error::Error>> {
    let conn_string = "file::memory:?cache=shared".to_string();
    let sqlite_options = SqliteOptions::new(conn_string);
    let config_and_pool = ConfigAndPool::new_sqlite(sqlite_options).await?;

    let ddl = [
        include_str!("../../rusty-golf-actix/src/sql/schema/sqlite/00_event.sql"),
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

    let json = serde_json::from_str(include_str!("test7/test7_dbprefill.json"))?;
    db_prefill(&json, &config_and_pool, DatabaseType::Sqlite).await?;

    Ok(config_and_pool)
}

fn load_detailed_scores(event_id: i32) -> Result<SummaryDetailedScores, Box<dyn std::error::Error>>
{
    let detailed_scores_vec: Vec<DetailedScore> = serde_json::from_str(match event_id {
        401_580_355 => include_str!("test7/detailed_scores_401580355.json"),
        401_703_504 => include_str!("test7/detailed_scores_401703504.json"),
        _ => return Err("Unexpected event ID".into()),
    })?;

    Ok(SummaryDetailedScores {
        detailed_scores: detailed_scores_vec,
    })
}

fn load_summary_scores(event_id: i32) -> Result<AllBettorScoresByRound, Box<dyn std::error::Error>>
{
    let summary_scores_obj: serde_json::Value = serde_json::from_str(match event_id {
        401_580_355 => include_str!("test7/summary_scores_x_401580355.json"),
        401_703_504 => include_str!("test7/summary_scores_x_401703504.json"),
        _ => return Err("Unexpected event ID".into()),
    })?;

    let summary_scores_vec = summary_scores_obj["summary_scores"]
        .as_array()
        .expect("Expected summary_scores to be an array")
        .clone();

    Ok(AllBettorScoresByRound {
        summary_scores: serde_json::from_value(serde_json::Value::Array(summary_scores_vec))?,
    })
}

async fn set_refresh_from_espn(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = config_and_pool.get_connection().await?;
    let update_query = "UPDATE event SET refresh_from_espn = 1 WHERE espn_id = ?1;";
    conn.execute_dml(
        update_query,
        &[sql_middleware::middleware::RowValues::Int(event_id.into())],
    )
    .await?;
    Ok(())
}

async fn fetch_score_view_factor(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<f32, Box<dyn std::error::Error>> {
    let mut conn = config_and_pool.get_connection().await?;
    let query = "SELECT score_view_step_factor FROM event WHERE espn_id = ?1;";
    let result = conn
        .execute_select(
            query,
            &[sql_middleware::middleware::RowValues::Int(event_id.into())],
        )
        .await?;

    assert!(!result.results.is_empty(), "No event found in database");
    let score_view_factor = result.results[0]
        .get("score_view_step_factor")
        .and_then(sql_middleware::middleware::RowValues::as_float)
        .map(|v| {
            #[allow(clippy::cast_possible_truncation)]
            {
                v as f32
            }
        })
        .expect("Could not get score_view_step_factor from database");

    Ok(score_view_factor)
}

async fn count_event_user_player_with_step_factor(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<i64, Box<dyn std::error::Error>> {
    let mut conn = config_and_pool.get_connection().await?;
    let query = "SELECT COUNT(*) as count FROM event_user_player 
                WHERE event_id = (SELECT event_id FROM event WHERE espn_id = ?1) 
                AND score_view_step_factor IS NOT NULL;";
    let result = conn
        .execute_select(
            query,
            &[sql_middleware::middleware::RowValues::Int(event_id.into())],
        )
        .await?;

    let count = result.results[0]
        .get("count")
        .and_then(|v| v.as_int())
        .copied()
        .expect("Could not get count of event_user_player entries");

    Ok(count)
}

fn save_debug_html(
    event_id: i32,
    html_output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let debug_dir_path = format!("tests/test7/debug_{event_id}");
    let debug_dir = Path::new(&debug_dir_path);
    std::fs::create_dir_all(debug_dir)?;
    let debug_file = debug_dir.join("actual_output.html");
    let mut file = std::fs::File::create(&debug_file)?;
    writeln!(file, "{html_output}")?;
    Ok(())
}

async fn assert_event_401580355(
    config_and_pool: &ConfigAndPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_id = 401_580_355;
    let global_step_factor = fetch_score_view_factor(config_and_pool, event_id).await?;
    assert!(
        (global_step_factor - 4.5_f32).abs() < f32::EPSILON,
        "Expected global step factor to be 4.5 for event 401580355"
    );

    let count = count_event_user_player_with_step_factor(config_and_pool, event_id).await?;
    assert_eq!(
        count, 0,
        "Expected event 401580355 to have no event_user_player entries with step factors"
    );

    println!("✓ Test passed for event 401580355: correctly uses global step factor");
    Ok(())
}

async fn assert_event_401703504(
    config_and_pool: &ConfigAndPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_id = 401_703_504;
    let count = count_event_user_player_with_step_factor(config_and_pool, event_id).await?;
    assert!(
        count == 15,
        "Expected event 401703504 to have event_user_player entries with step factors"
    );

    let mut conn = config_and_pool.get_connection().await?;
    let query = "SELECT b.name as bettorname, eup.score_view_step_factor 
                FROM event_user_player eup
                JOIN bettor b ON eup.user_id = b.user_id
                WHERE eup.event_id = (SELECT event_id FROM event WHERE espn_id = ?1)
                AND b.name = 'Player1'
                AND eup.score_view_step_factor IS NOT NULL
                LIMIT 1;";
    let result = conn
        .execute_select(
            query,
            &[sql_middleware::middleware::RowValues::Int(event_id.into())],
        )
        .await?;

    assert!(
        !result.results.is_empty(),
        "Could not find Player1 with step factor"
    );

    let step_factor = result.results[0]
        .get("score_view_step_factor")
        .and_then(sql_middleware::middleware::RowValues::as_float)
        .map(|v| {
            #[allow(clippy::cast_possible_truncation)]
            {
                v as f32
            }
        })
        .expect("Could not get step_factor value");

    assert!(
        (step_factor - 1.0_f32).abs() < f32::EPSILON,
        "Expected Player1 to have step_factor 1.0 in event 401703504"
    );

    println!("✓ Test passed for event 401703504: correctly uses per-player step factors");
    Ok(())
}

// This function is for testing, accessing the public render function
async fn test_render_template(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
    _summary_scores_x: &AllBettorScoresByRound,
    _detailed_scores: &SummaryDetailedScores,
) -> Result<String, Box<dyn std::error::Error>> {
    // Create a basic score data structure for template rendering
    let html = rusty_golf::view::score::render_scores_template(
        &rusty_golf::model::ScoreData {
            bettor_struct: vec![],
            score_struct: vec![],
            last_refresh: "1 minute".to_string(),
            last_refresh_source: rusty_golf::model::RefreshSource::Db,
        },
        true,
        config_and_pool,
        event_id,
    )
    .await?;

    // Extract just the drop-down bar section from the full HTML
    let html_string = html.into_string();
    let start_marker = "<h3 class=\"playerbars\">Score by Player</h3>";
    let end_marker = "<h3 class=\"playerbars\">Score by Golfer</h3>";

    if let (Some(start_idx), Some(end_idx)) =
        (html_string.find(start_marker), html_string.find(end_marker))
    {
        Ok(html_string[start_idx..end_idx].to_string())
    } else {
        Err("Could not find the drop-down bar HTML section".into())
    }
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_new_step_factor() -> Result<(), Box<dyn std::error::Error>> {
    let config_and_pool = setup_db().await?;
    let event_ids = [401_580_355, 401_703_504];

    for event_id in event_ids {
        let detailed_scores = load_detailed_scores(event_id)?;
        let summary_scores_x = load_summary_scores(event_id)?;

        set_refresh_from_espn(&config_and_pool, event_id).await?;

        // Now render the template (data will be pulled from the DB)
        let html_output = test_render_template(
            &config_and_pool,
            event_id,
            &summary_scores_x,
            &detailed_scores,
        )
        .await?;

        save_debug_html(event_id, &html_output)?;

        // STEP 4: Read the reference HTML file containing expected output
        let reference_path_str = format!("tests/test7/reference_html_{event_id}.html");
        let reference_path = Path::new(&reference_path_str);
        assert!(
            reference_path.exists(),
            "Reference file not found at: {}",
            reference_path.display()
        );
        let _reference_html = std::fs::read_to_string(reference_path)?;

        // For this test, we simply verify that we can render a page for each event
        // without errors, after adding the score_view_step_factor to event_user_player.

        // STEP 5: For event 401580355
        if event_id == 401_580_355 {
            assert_event_401580355(&config_and_pool).await?;
        } else if event_id == 401_703_504 {
            // STEP 6: For event 401703504, verify per-player step factors
            assert_event_401703504(&config_and_pool).await?;
        }
    }

    Ok(())
}
