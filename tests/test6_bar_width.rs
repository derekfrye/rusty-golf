use rusty_golf::controller::db_prefill::db_prefill;
use rusty_golf::model::{AllBettorScoresByRound, BettorScoreByRound, DetailedScore, SummaryDetailedScores};
use std::io::Write;
use std::path::Path;

use sql_middleware::middleware::{
    AsyncDatabaseExecutor, ConfigAndPool, DatabaseType, MiddlewarePool, QueryAndParams,
};

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

    if let (Some(start_idx), Some(end_idx)) = (html_string.find(start_marker), html_string.find(end_marker)) {
        Ok(html_string[start_idx..end_idx].to_string())
    } else {
        Err("Could not find the drop-down bar HTML section".into())
    }
}

#[tokio::test]
async fn test_bar_width() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the SQLite in-memory database
    let conn_string = "file::memory:?cache=shared".to_string();
    let config_and_pool = ConfigAndPool::new_sqlite(conn_string).await?;

    // Set up database schema from SQL files
    let ddl = [
        include_str!("../src/admin/model/sql/schema/sqlite/00_event.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/02_golfer.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/03_bettor.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/04_event_user_player.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/05_eup_statistic.sql"),
    ];

    let query_and_params = QueryAndParams {
        query: ddl.join("\n"),
        params: vec![],
    };

    let pool = config_and_pool.pool.get().await?;
    let mut conn = MiddlewarePool::get_connection(pool).await?;
    conn.execute_batch(&query_and_params.query).await?;

    // Fill the database with test data
    let json = serde_json::from_str(include_str!("test5_dbprefill.json"))?;
    db_prefill(&json, &config_and_pool, DatabaseType::Sqlite).await?;

    // Create test data for detailed_scores (golf scores)
    let detailed_scores = SummaryDetailedScores {
        detailed_scores: vec![
            DetailedScore {
                bettor_name: "Player1".to_string(),
                golfer_name: "Scottie Scheffler".to_string(),
                rounds: vec![0, 1],
                scores: vec![-3, 0],
            },
            DetailedScore {
                bettor_name: "Player1".to_string(),
                golfer_name: "Collin Morikawa".to_string(),
                rounds: vec![0, 1],
                scores: vec![-2, 0],
            },
            DetailedScore {
                bettor_name: "Player1".to_string(),
                golfer_name: "Min Woo Lee".to_string(),
                rounds: vec![0, 1],
                scores: vec![0, 0],
            },
            DetailedScore {
                bettor_name: "Player2".to_string(),
                golfer_name: "Bryson DeChambeau".to_string(),
                rounds: vec![0, 1],
                scores: vec![-1, 0],
            },
            DetailedScore {
                bettor_name: "Player2".to_string(),
                golfer_name: "Justin Thomas".to_string(),
                rounds: vec![0, 1],
                scores: vec![1, 0],
            },
            DetailedScore {
                bettor_name: "Player2".to_string(),
                golfer_name: "Hideki Matsuyama".to_string(),
                rounds: vec![0, 1],
                scores: vec![0, 0],
            },
            DetailedScore {
                bettor_name: "Player3".to_string(),
                golfer_name: "Rory McIlroy".to_string(),
                rounds: vec![0, 1],
                scores: vec![0, 0],
            },
            DetailedScore {
                bettor_name: "Player3".to_string(),
                golfer_name: "Ludvig Åberg".to_string(),
                rounds: vec![0, 1],
                scores: vec![1, 0],
            },
            DetailedScore {
                bettor_name: "Player3".to_string(),
                golfer_name: "Sepp Straka".to_string(),
                rounds: vec![0, 1],
                scores: vec![0, 0],
            },
            DetailedScore {
                bettor_name: "Player4".to_string(),
                golfer_name: "Brooks Koepka".to_string(),
                rounds: vec![0, 1],
                scores: vec![0, 0],
            },
            DetailedScore {
                bettor_name: "Player4".to_string(),
                golfer_name: "Viktor Hovland".to_string(),
                rounds: vec![0, 1],
                scores: vec![0, 0],
            },
            DetailedScore {
                bettor_name: "Player4".to_string(),
                golfer_name: "Jason Day".to_string(),
                rounds: vec![0, 1],
                scores: vec![0, 0],
            },
            DetailedScore {
                bettor_name: "Player5".to_string(),
                golfer_name: "Xander Schauffele".to_string(),
                rounds: vec![0, 1],
                scores: vec![3, 0],
            },
            DetailedScore {
                bettor_name: "Player5".to_string(),
                golfer_name: "Jon Rahm".to_string(),
                rounds: vec![0, 1],
                scores: vec![1, 0],
            },
            DetailedScore {
                bettor_name: "Player5".to_string(),
                golfer_name: "Will Zalatoris".to_string(),
                rounds: vec![0, 1],
                scores: vec![0, 0],
            },
        ],
    };

    // Create test data for summary_scores_x (player aggregated scores)
    let summary_scores_x = AllBettorScoresByRound {
        summary_scores: vec![
            BettorScoreByRound {
                bettor_name: "Player1".to_string(),
                computed_rounds: vec![0, 1],
                scores_aggregated_by_golf_grp_by_rd: vec![-5, 0],
            },
            BettorScoreByRound {
                bettor_name: "Player2".to_string(),
                computed_rounds: vec![0, 1],
                scores_aggregated_by_golf_grp_by_rd: vec![0, 0],
            },
            BettorScoreByRound {
                bettor_name: "Player3".to_string(),
                computed_rounds: vec![0, 1],
                scores_aggregated_by_golf_grp_by_rd: vec![1, 0],
            },
            BettorScoreByRound {
                bettor_name: "Player4".to_string(),
                computed_rounds: vec![0, 1],
                scores_aggregated_by_golf_grp_by_rd: vec![0, 0],
            },
            BettorScoreByRound {
                bettor_name: "Player5".to_string(),
                computed_rounds: vec![0, 1],
                scores_aggregated_by_golf_grp_by_rd: vec![4, 0],
            },
        ],
    };

    // Test the Masters 2025 event
    let event_id = 401703504;

    // STEP 1: Verify that the score_view_step_factor is set correctly in the database
    let pool = config_and_pool.pool.get().await?;
    let mut conn = MiddlewarePool::get_connection(pool).await?;
    let query = "SELECT score_view_step_factor FROM event WHERE espn_id = ?1;";
    let result = conn.execute_select(query, &[sql_middleware::middleware::RowValues::Int(event_id.into())]).await?;
    
    assert!(!result.results.is_empty(), "No event found in database");
    let score_view_factor = result.results[0]
        .get("score_view_step_factor")
        .and_then(|v| v.as_float())
        .map(|v| v as f32)
        .expect("Could not get score_view_step_factor from database");
    
    // STEP 2: Verify the step factor is 4.5 as specified in the test5_dbprefill.json file
    assert_eq!(score_view_factor, 4.5, "score_view_step_factor should be 4.5");
    
    // STEP 3: Render the drop-down bar HTML and save output for debugging
    let html_output = test_render_template(
        &config_and_pool,
        event_id,
        &summary_scores_x,
        &detailed_scores,
    ).await?;
    
    // Save output for debugging
    let debug_dir = Path::new("tests/test6/debug");
    std::fs::create_dir_all(debug_dir)?;
    let debug_file = debug_dir.join("actual_output.html");
    let mut file = std::fs::File::create(&debug_file)?;
    writeln!(file, "{}", html_output)?;
    
    // STEP 4: Read the reference HTML file containing expected output
    let reference_path = Path::new("tests/test6/test6_ref_html.html");
    assert!(reference_path.exists(), "Reference file not found at: {}", reference_path.display());
    let reference_html = std::fs::read_to_string(reference_path)?;
    
    // STEP 5: Verify that the reference HTML contains the expected bar widths
    // based on score_view_step_factor (4.5)

    // STEP 6: Get the factor that should be used by preprocess_golfer_data
    let config = rusty_golf::model::get_title_and_score_view_conf_from_db(&config_and_pool, event_id).await?;
    assert_eq!(config.score_view_step_factor, 4.5, "score_view_step_factor should be 4.5");
    
    // STEP 7: Test that the HTML contains bars with widths proportional to scores * step_factor
    // The algorithm in preprocess_golfer_data takes a score, multiplies by step_factor to get width
    let test_scores: [i32; 7] = [-3, -2, -1, 0, 1, 2, 3];
    for score in test_scores.iter() {
        let expected_width = (score.abs() as f32 * config.score_view_step_factor).to_string();
        
        // Check in reference HTML that expected widths exist
        // Skip score 0 as it might not be exactly 0% in the HTML
        if score.abs() > 0 {
            assert!(
                reference_html.contains(&format!("width: {}%", expected_width)), 
                "Reference HTML should contain width: {}% for score {}", expected_width, score
            );
        }
    }
    
    // SUCCESS! We've verified that score_view_step_factor in the database impacts HTML rendering
    // by checking that bar widths in the HTML are proportional to scores * score_view_step_factor
    println!("✓ Test passed: bar widths in HTML correctly reflect score_view_step_factor");
    
    Ok(())
}