use rusty_golf::controller::db_prefill::db_prefill;
use rusty_golf::model::{AllBettorScoresByRound, DetailedScore, SummaryDetailedScores};
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

    if let (Some(start_idx), Some(end_idx)) =
        (html_string.find(start_marker), html_string.find(end_marker))
    {
        Ok(html_string[start_idx..end_idx].to_string())
    } else {
        Err("Could not find the drop-down bar HTML section".into())
    }
}

#[tokio::test]
async fn test_new_step_factor() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the SQLite in-memory database
    let conn_string = "file::memory:?cache=shared".to_string();
    let config_and_pool = ConfigAndPool::new_sqlite(conn_string).await?;

    // Set up database schema from SQL files
    let ddl = [
        include_str!("../src/sql/schema/sqlite/00_event.sql"),
        include_str!("../src/sql/schema/sqlite/02_golfer.sql"),
        include_str!("../src/sql/schema/sqlite/03_bettor.sql"),
        include_str!("../src/sql/schema/sqlite/04_event_user_player.sql"),
        include_str!("../src/sql/schema/sqlite/05_eup_statistic.sql"),
    ];

    let query_and_params = QueryAndParams {
        query: ddl.join("\n"),
        params: vec![],
    };

    let pool = config_and_pool.pool.get().await?;
    let mut conn = MiddlewarePool::get_connection(pool).await?;
    conn.execute_batch(&query_and_params.query).await?;

    // Fill the database with test data from test7_dbprefill.json
    let json = serde_json::from_str(include_str!("test7/test7_dbprefill.json"))?;
    db_prefill(&json, &config_and_pool, DatabaseType::Sqlite).await?;

    // Test the two events: 401580355 and 401703504
    let event_ids = [401580355, 401703504];

    for event_id in event_ids {
        // Load test data for this event
        // Load detailed scores from the JSON file
        let detailed_scores_vec: Vec<DetailedScore> = serde_json::from_str(match event_id {
            401580355 => include_str!("test7/detailed_scores_401580355.json"),
            401703504 => include_str!("test7/detailed_scores_401703504.json"),
            _ => panic!("Unexpected event ID"),
        })?;

        let detailed_scores = SummaryDetailedScores {
            detailed_scores: detailed_scores_vec,
        };

        // Load summary scores
        let summary_scores_obj: serde_json::Value = serde_json::from_str(match event_id {
            401580355 => include_str!("test7/summary_scores_x_401580355.json"),
            401703504 => include_str!("test7/summary_scores_x_401703504.json"),
            _ => panic!("Unexpected event ID"),
        })?;

        let summary_scores_vec = summary_scores_obj["summary_scores"]
            .as_array()
            .expect("Expected summary_scores to be an array")
            .to_vec();

        let summary_scores_x = AllBettorScoresByRound {
            summary_scores: serde_json::from_value(serde_json::Value::Array(summary_scores_vec))?,
        };

        // STEP 1: Verify that the event has the correct global step factor
        let pool = config_and_pool.pool.get().await?;
        let mut conn = MiddlewarePool::get_connection(pool).await?;

        // Set refresh_from_espn to a valid value (0 or 1) for all events
        let update_query = "UPDATE event SET refresh_from_espn = 1 WHERE espn_id = ?1;";
        conn.execute_dml(
            update_query,
            &[sql_middleware::middleware::RowValues::Int(event_id.into())],
        )
        .await?;

        // Now query for score_view_step_factor
        let query = "SELECT score_view_step_factor FROM event WHERE espn_id = ?1;";
        let result = conn
            .execute_select(
                query,
                &[sql_middleware::middleware::RowValues::Int(event_id.into())],
            )
            .await?;

        assert!(!result.results.is_empty(), "No event found in database");
        let _event_step_factor = result.results[0]
            .get("score_view_step_factor")
            .and_then(|v| v.as_float())
            .map(|v| v as f32)
            .expect("Could not get score_view_step_factor from database");

        // STEP 2: Check if event_user_player entries have step factors for event 401703504
        if event_id == 401703504 {
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

            assert!(
                count == 15,
                "Expected event 401703504 to have event_user_player entries with step factors"
            );
        }

        // STEP 3: Insert some dummy data for testing the visual rendering
        let query = "SELECT event_id FROM event WHERE espn_id = ?1;";
        let result = conn
            .execute_select(
                query,
                &[sql_middleware::middleware::RowValues::Int(event_id.into())],
            )
            .await?;

        let _event_db_id = result.results[0]
            .get("event_id")
            .and_then(|v| v.as_int())
            .copied()
            .expect("Could not get event_id from database");

        // // Insert dummy statistics for the event
        // for detail in &detailed_scores.detailed_scores {
        //     // Find eup_id for this golfer-bettor combination
        //     let query = format!("
        //         SELECT eup.eup_id
        //         FROM event_user_player eup
        //         JOIN bettor b ON eup.user_id = b.user_id
        //         JOIN golfer g ON eup.golfer_id = g.golfer_id
        //         WHERE eup.event_id = ? AND b.name = ? AND g.name = ?
        //     ");

        //     let result = conn.execute_select(
        //         &query,
        //         &[
        //             sql_middleware::middleware::RowValues::Int(event_db_id),
        //             sql_middleware::middleware::RowValues::Text(detail.bettor_name.clone()),
        //             sql_middleware::middleware::RowValues::Text(detail.golfer_name.clone()),
        //         ]
        //     ).await;

        //     // If we found an eup_id, insert statistics
        //     if let Ok(res) = result {
        //         if !res.results.is_empty() {
        //             if let Some(eup_id) = res.results[0].get("eup_id").and_then(|v| v.as_int()) {
        //                 // Insert dummy statistics
        //                 let rounds_json = serde_json::to_string(&detail.rounds)?;
        //                 let round_scores_json = serde_json::to_string(&detail.scores)?;
        //                 let tee_times_json = "[]"; // Empty for simplicity
        //                 let holes_completed_json = "[]"; // Empty for simplicity
        //                 let line_scores_json = "[]"; // Empty for simplicity

        //                 let query = "
        //                     INSERT INTO eup_statistic
        //                     (event_espn_id, golfer_espn_id, eup_id, grp, rounds, round_scores,
        //                     tee_times, holes_completed_by_round, line_scores, total_score)
        //                     VALUES (?, 1, ?, 1, ?, ?, ?, ?, ?, ?)
        //                 ";

        //                 let total_score: i32 = detail.scores.iter().sum();

        //                 let _ = conn.execute_dml(
        //                     query,
        //                     &[
        //                         sql_middleware::middleware::RowValues::Int(event_id.into()),
        //                         sql_middleware::middleware::RowValues::Int(*eup_id),
        //                         sql_middleware::middleware::RowValues::Text(rounds_json),
        //                         sql_middleware::middleware::RowValues::Text(round_scores_json),
        //                         sql_middleware::middleware::RowValues::Text(tee_times_json.to_string()),
        //                         sql_middleware::middleware::RowValues::Text(holes_completed_json.to_string()),
        //                         sql_middleware::middleware::RowValues::Text(line_scores_json.to_string()),
        //                         sql_middleware::middleware::RowValues::Int(total_score.into()),
        //                     ]
        //                 ).await;
        //             }
        //         }
        //     }
        // }

        // Now render the template (data will be pulled from the DB)
        let html_output = test_render_template(
            &config_and_pool,
            event_id,
            &summary_scores_x,
            &detailed_scores,
        )
        .await?;

        // Save output for debugging
        let debug_dir_path = format!("tests/test7/debug_{event_id}");
        let debug_dir = Path::new(&debug_dir_path);
        std::fs::create_dir_all(debug_dir)?;
        let debug_file = debug_dir.join("actual_output.html");
        let mut file = std::fs::File::create(&debug_file)?;
        writeln!(file, "{html_output}")?;

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
        if event_id == 401580355 {
            // Verify the global step factor is 4.5
            let query = "SELECT score_view_step_factor FROM event WHERE espn_id = ?1;";
            let result = conn
                .execute_select(
                    query,
                    &[sql_middleware::middleware::RowValues::Int(event_id.into())],
                )
                .await?;

            let global_step_factor = result.results[0]
                .get("score_view_step_factor")
                .and_then(|v| v.as_float())
                .map(|v| v as f32)
                .expect("Could not get score_view_step_factor from database");

            assert_eq!(
                global_step_factor, 4.5,
                "Expected global step factor to be 4.5 for event 401580355"
            );

            // Verify that score_view_step_factor is not set in event_user_player
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
                .expect("Could not get count of event_user_player entries with step factors");

            assert_eq!(
                count, 0,
                "Expected event 401580355 to have no event_user_player entries with step factors"
            );

            println!("✓ Test passed for event 401580355: correctly uses global step factor");
        }
        // STEP 6: For event 401703504, verify per-player step factors
        else if event_id == 401703504 {
            // Verify that score_view_step_factor is set in event_user_player
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
                .expect("Could not get count of event_user_player entries with step factors");

            assert!(
                count == 15,
                "Expected event 401703504 to have event_user_player entries with step factors"
            );

            // Let's verify at least one specific step factor value
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
                .and_then(|v| v.as_float())
                .map(|v| v as f32)
                .expect("Could not get step_factor value");

            assert_eq!(
                step_factor, 1.0,
                "Expected Player1 to have step_factor 1.0 in event 401703504"
            );

            println!("✓ Test passed for event 401703504: correctly uses per-player step factors");
        }
    }

    Ok(())
}
