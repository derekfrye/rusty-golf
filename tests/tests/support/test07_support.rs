use crate::common::ConnExt;
use rusty_golf_actix::controller::db_prefill::db_prefill;
use rusty_golf_actix::model::{AllBettorScoresByRound, DetailedScore, SummaryDetailedScores};
use sql_middleware::middleware::{ConfigAndPool, DatabaseType, SqliteOptions};
use std::io::Write;
use std::path::Path;

pub(super) async fn setup_db() -> Result<ConfigAndPool, Box<dyn std::error::Error>> {
    let sqlite_options = SqliteOptions::new("file::memory:?cache=shared".to_string());
    let config_and_pool = ConfigAndPool::new_sqlite(sqlite_options).await?;
    let ddl = [
        include_str!("../../../actix/src/sql/schema/sqlite/00_event.sql"),
        include_str!("../../../actix/src/sql/schema/sqlite/02_golfer.sql"),
        include_str!("../../../actix/src/sql/schema/sqlite/03_bettor.sql"),
        include_str!("../../../actix/src/sql/schema/sqlite/04_event_user_player.sql"),
        include_str!("../../../actix/src/sql/schema/sqlite/05_eup_statistic.sql"),
    ];
    let mut conn = config_and_pool.get_connection().await?;
    conn.execute_batch(&ddl.join("\n")).await?;
    let json = serde_json::from_str(include_str!("../test07/test07_dbprefill.json"))?;
    db_prefill(&json, &config_and_pool, DatabaseType::Sqlite).await?;
    Ok(config_and_pool)
}

pub(super) fn load_detailed_scores(
    event_id: i32,
) -> Result<SummaryDetailedScores, Box<dyn std::error::Error>> {
    let detailed_scores: Vec<DetailedScore> = serde_json::from_str(match event_id {
        401_580_355 => include_str!("../test07/detailed_scores_401580355.json"),
        401_703_504 => include_str!("../test07/detailed_scores_401703504.json"),
        _ => return Err("Unexpected event ID".into()),
    })?;
    Ok(SummaryDetailedScores { detailed_scores })
}

pub(super) fn load_summary_scores(
    event_id: i32,
) -> Result<AllBettorScoresByRound, Box<dyn std::error::Error>> {
    let summary_scores_obj: serde_json::Value = serde_json::from_str(match event_id {
        401_580_355 => include_str!("../test07/summary_scores_x_401580355.json"),
        401_703_504 => include_str!("../test07/summary_scores_x_401703504.json"),
        _ => return Err("Unexpected event ID".into()),
    })?;
    let summary_scores = summary_scores_obj["summary_scores"]
        .as_array()
        .expect("Expected summary_scores to be an array")
        .clone();
    Ok(AllBettorScoresByRound {
        summary_scores: serde_json::from_value(serde_json::Value::Array(summary_scores))?,
    })
}

pub(super) async fn set_refresh_from_espn(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = config_and_pool.get_connection().await?;
    conn.execute_dml(
        "UPDATE event SET refresh_from_espn = 1 WHERE espn_id = ?1;",
        &[sql_middleware::middleware::RowValues::Int(event_id.into())],
    )
    .await?;
    Ok(())
}

pub(super) fn save_debug_html(
    event_id: i32,
    html_output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let debug_dir_path = format!("tests/test07/debug_{event_id}");
    let debug_dir = Path::new(&debug_dir_path);
    std::fs::create_dir_all(debug_dir)?;
    let mut file = std::fs::File::create(debug_dir.join("actual_output.html"))?;
    writeln!(file, "{html_output}")?;
    Ok(())
}

pub(super) async fn assert_event_401580355(
    config_and_pool: &ConfigAndPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_id = 401_580_355;
    let global_step_factor = fetch_score_view_factor(config_and_pool, event_id).await?;
    assert!(
        (global_step_factor - 4.5_f32).abs() < f32::EPSILON,
        "Expected global step factor to be 4.5 for event 401580355"
    );
    let count = count_event_user_player_with_step_factor(config_and_pool, event_id).await?;
    assert_eq!(count, 0);
    Ok(())
}

pub(super) async fn assert_event_401703504(
    config_and_pool: &ConfigAndPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_id = 401_703_504;
    let count = count_event_user_player_with_step_factor(config_and_pool, event_id).await?;
    assert_eq!(count, 15);
    let step_factor = fetch_player_step_factor(config_and_pool, event_id).await?;
    assert!(
        (step_factor - 1.0_f32).abs() < f32::EPSILON,
        "Expected Player1 to have step_factor 1.0 in event 401703504"
    );
    Ok(())
}

pub(super) async fn test_render_template(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
    _summary_scores_x: &AllBettorScoresByRound,
    _detailed_scores: &SummaryDetailedScores,
) -> Result<String, Box<dyn std::error::Error>> {
    let html = rusty_golf_actix::view::score::render_scores_template(
        &rusty_golf_actix::model::ScoreData {
            bettor_struct: vec![],
            score_struct: vec![],
            last_refresh: "1 minute".to_string(),
            last_refresh_source: rusty_golf_actix::model::RefreshSource::Db,
            cache_hit: true,
        },
        true,
        config_and_pool,
        event_id,
    )
    .await?;
    Ok(extract_player_bar_section(&html.into_string()))
}

async fn fetch_score_view_factor(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<f32, Box<dyn std::error::Error>> {
    let value = query_float(
        config_and_pool,
        "SELECT score_view_step_factor FROM event WHERE espn_id = ?1;",
        event_id,
        "score_view_step_factor",
    )
    .await?;
    Ok(value.to_string().parse()?)
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
    Ok(result.results[0]
        .get("count")
        .and_then(sql_middleware::middleware::RowValues::as_int)
        .copied()
        .expect("Could not get count of event_user_player entries"))
}

async fn fetch_player_step_factor(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<f32, Box<dyn std::error::Error>> {
    let query = "SELECT eup.score_view_step_factor 
                FROM event_user_player eup
                JOIN bettor b ON eup.user_id = b.user_id
                WHERE eup.event_id = (SELECT event_id FROM event WHERE espn_id = ?1)
                AND b.name = 'Player1'
                AND eup.score_view_step_factor IS NOT NULL
                LIMIT 1;";
    let value = query_float(config_and_pool, query, event_id, "score_view_step_factor").await?;
    Ok(value.to_string().parse()?)
}

async fn query_float(
    config_and_pool: &ConfigAndPool,
    query: &str,
    event_id: i32,
    key: &str,
) -> Result<f64, Box<dyn std::error::Error>> {
    let mut conn = config_and_pool.get_connection().await?;
    let result = conn
        .execute_select(
            query,
            &[sql_middleware::middleware::RowValues::Int(event_id.into())],
        )
        .await?;
    Ok(result.results[0]
        .get(key)
        .and_then(sql_middleware::middleware::RowValues::as_float)
        .expect("Could not get float value"))
}

fn extract_player_bar_section(html_string: &str) -> String {
    let start_marker = "<h3 class=\"playerbars\">Score by Player</h3>";
    let end_marker = "<h3 class=\"playerbars\">Score by Golfer</h3>";
    if let (Some(start_idx), Some(end_idx)) =
        (html_string.find(start_marker), html_string.find(end_marker))
    {
        return html_string[start_idx..end_idx].to_string();
    }
    html_string.to_string()
}
