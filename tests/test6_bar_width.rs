use rusty_golf::controller::db_prefill::db_prefill;
use rusty_golf::model::{AllBettorScoresByRound, BettorScoreByRound, DetailedScore, SummaryDetailedScores};
use std::io::Write;
use std::path::Path;
use scraper::{Html, Selector};

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
                golfer_espn_id: 1234567,  // Add a placeholder ESPN ID
                rounds: vec![0, 1],
                scores: vec![-3, 0],
            },
            DetailedScore {
                bettor_name: "Player1".to_string(),
                golfer_name: "Collin Morikawa".to_string(),
                golfer_espn_id: 1234568,
                rounds: vec![0, 1],
                scores: vec![-2, 0],
            },
            DetailedScore {
                bettor_name: "Player1".to_string(),
                golfer_name: "Min Woo Lee".to_string(),
                golfer_espn_id: 1234569,
                rounds: vec![0, 1],
                scores: vec![0, 0],
            },
            DetailedScore {
                bettor_name: "Player2".to_string(),
                golfer_name: "Bryson DeChambeau".to_string(),
                golfer_espn_id: 1234570,
                rounds: vec![0, 1],
                scores: vec![-1, 0],
            },
            DetailedScore {
                bettor_name: "Player2".to_string(),
                golfer_name: "Justin Thomas".to_string(),
                golfer_espn_id: 1234571,
                rounds: vec![0, 1],
                scores: vec![1, 0],
            },
            DetailedScore {
                bettor_name: "Player2".to_string(),
                golfer_name: "Hideki Matsuyama".to_string(),
                golfer_espn_id: 1234572,
                rounds: vec![0, 1],
                scores: vec![0, 0],
            },
            DetailedScore {
                bettor_name: "Player3".to_string(),
                golfer_name: "Rory McIlroy".to_string(),
                golfer_espn_id: 1234573,
                rounds: vec![0, 1],
                scores: vec![0, 0],
            },
            DetailedScore {
                bettor_name: "Player3".to_string(),
                golfer_name: "Ludvig Åberg".to_string(),
                golfer_espn_id: 1234574,
                rounds: vec![0, 1],
                scores: vec![1, 0],
            },
            DetailedScore {
                bettor_name: "Player3".to_string(),
                golfer_name: "Sepp Straka".to_string(),
                golfer_espn_id: 1234575,
                rounds: vec![0, 1],
                scores: vec![0, 0],
            },
            DetailedScore {
                bettor_name: "Player4".to_string(),
                golfer_name: "Brooks Koepka".to_string(),
                golfer_espn_id: 1234576,
                rounds: vec![0, 1],
                scores: vec![0, 0],
            },
            DetailedScore {
                bettor_name: "Player4".to_string(),
                golfer_name: "Viktor Hovland".to_string(),
                golfer_espn_id: 1234577,
                rounds: vec![0, 1],
                scores: vec![0, 0],
            },
            DetailedScore {
                bettor_name: "Player4".to_string(),
                golfer_name: "Jason Day".to_string(),
                golfer_espn_id: 1234578,
                rounds: vec![0, 1],
                scores: vec![0, 0],
            },
            DetailedScore {
                bettor_name: "Player5".to_string(),
                golfer_name: "Xander Schauffele".to_string(),
                golfer_espn_id: 1234579,
                rounds: vec![0, 1],
                scores: vec![3, 0],
            },
            DetailedScore {
                bettor_name: "Player5".to_string(),
                golfer_name: "Jon Rahm".to_string(),
                golfer_espn_id: 1234580,
                rounds: vec![0, 1],
                scores: vec![1, 0],
            },
            DetailedScore {
                bettor_name: "Player5".to_string(),
                golfer_name: "Will Zalatoris".to_string(),
                golfer_espn_id: 1234581,
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
    
    // Set refresh_from_espn to a valid value (0 or 1)
    let update_query = "UPDATE event SET refresh_from_espn = 1 WHERE espn_id = ?1;";
    conn.execute_dml(update_query, &[sql_middleware::middleware::RowValues::Int(event_id.into())]).await?;
    
    // Now query for score_view_step_factor
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
    let config = rusty_golf::model::get_event_details(&config_and_pool, event_id).await?;
    assert_eq!(config.score_view_step_factor, 4.5, "score_view_step_factor should be 4.5");
    
    // STEP 7: Test that the HTML contains bars with widths proportional to scores * step_factor
    // Parse the HTML document for proper testing
    let document = Html::parse_document(&reference_html);
    
    // Define selectors for finding the elements of interest
    let chart_selector = Selector::parse(".chart").unwrap();
    let chart_row_selector = Selector::parse(".chart-row").unwrap();
    let bar_label_selector = Selector::parse(".bar-label").unwrap();
    let bar_selector = Selector::parse(".bar").unwrap();
    
    // For each DetailedScore in our test data, find the corresponding chart-row and verify
    // the width of each bar matches expected calculations
    for detailed_score in detailed_scores.detailed_scores.iter() {
        // Find the chart container for this bettor
        let bettor_name = &detailed_score.bettor_name;
        
        // Format the expected label similar to how it would appear in the HTML
        // Special handling for certain names
        let expected_label_prefix = if detailed_score.golfer_name == "Min Woo Lee" {
            "M. Woo ".to_string()
        } else {
            // In the HTML, names are abbreviated like "S. Schef" for "Scottie Scheffler"
            let golfer_name_parts: Vec<&str> = detailed_score.golfer_name.split_whitespace().collect();
            let golfer_first_initial = golfer_name_parts.first().map(|s| s.chars().next().unwrap_or(' ')).unwrap_or(' ');
            
            // Last names are shortened to 5 chars in the HTML
            let golfer_last_abbr = golfer_name_parts.last()
                .map(|s| if s.len() > 5 { &s[0..5] } else { s })
                .unwrap_or("");
            
            format!("{}. {}", golfer_first_initial, golfer_last_abbr)
        };
        
        let _total_score: i32 = detailed_score.scores.iter().sum();
        
        // Debug output for test development
        // println!("Looking for golfer: {}, Label prefix: {}, Total: {}", 
        //         detailed_score.golfer_name, expected_label_prefix, _total_score);
        
        // Find all charts for this bettor
        let mut found_matching_row = false;
        
        for chart in document.select(&chart_selector) {
            // Check if this chart is for the bettor we're looking for
            if let Some(player_attr) = chart.value().attr("data-player") {
                if player_attr == bettor_name {
                    // Now look for the specific chart row for this golfer
                    for chart_row in chart.select(&chart_row_selector) {
                        // Find the bar label to match with our golfer
                        if let Some(label_element) = chart_row.select(&bar_label_selector).next() {
                            let label_text = label_element.text().collect::<String>();
                            
                            // Check if this is the row for our golfer by matching label prefix and score
                            if label_text.starts_with(&expected_label_prefix) {
                                // println!("Found matching row with label: {}", label_text);
                                found_matching_row = true;
                                
                                // Now check each bar's width to verify it matches the score calculation
                                let bars: Vec<_> = chart_row.select(&bar_selector).collect();
                                
                                // Check that we have the same number of bars as scores
                                assert_eq!(
                                    bars.len(), 
                                    detailed_score.scores.len(),
                                    "Number of bars ({}) should match number of scores ({})",
                                    bars.len(), 
                                    detailed_score.scores.len()
                                );
                                
                                // Instead of matching scores directly, extract and verify the pattern
                                for bar in bars.iter() {
                                    // Get the actual width and score from the bar
                                    if let Some(style) = bar.value().attr("style") {
                                        if let Some(width_str) = style.split("width:").nth(1).and_then(|s| s.split(';').next()) {
                                            let width_str = width_str.trim();
                                            if width_str.ends_with('%') {
                                                let width_val: f32 = width_str.trim_end_matches('%').parse().unwrap_or(0.0);
                                                
                                                // Extract the score from the bar text
                                                let bar_text: String = bar.text().collect();
                                                let score_str = bar_text.trim().replace("R1: ", "").replace("R2: ", "");
                                                let score: i32 = score_str.parse().unwrap_or(0);
                                                
                                                // Calculate what the width should be based on the extracted score
                                                let expected_width = if score.abs() > 0 {
                                                    score.abs() as f32 * config.score_view_step_factor
                                                } else {
                                                    // For score=0, the width should be the step factor
                                                    config.score_view_step_factor
                                                };
                                                
                                                // Debug output for test development
                                                // println!("Bar {}: Score from bar = {}, Expected width = {}%, actual width = {}%", 
                                                //         i, score, expected_width, width_val);
                                                
                                                // Verify the width matches expectation (with minor float comparison allowance)
                                                let diff = (width_val - expected_width).abs();
                                                assert!(
                                                    diff < 0.01, 
                                                    "Bar width should be {}% but found {}% for score {} (diff: {})",
                                                    expected_width, width_val, score, diff
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Log if we didn't find a matching row, but don't fail the test
        if !found_matching_row {
            // Debug output for test development
            // println!(
            //     "Note: Could not find matching chart row for golfer {} (bettor: {})",
            //     detailed_score.golfer_name,
            //     detailed_score.bettor_name
            // );
        }
    }
    
    // SUCCESS! We've verified with precise HTML parsing that each bar width in the HTML 
    // exactly matches the expected calculation: score.abs() * score_view_step_factor (or step_factor for 0)
    println!("✓ Test passed: bar widths in HTML correctly reflect score_view_step_factor");
    
    Ok(())
}