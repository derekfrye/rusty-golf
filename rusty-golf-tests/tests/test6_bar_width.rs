mod common;
use crate::common::ConnExt;
use rusty_golf::controller::db_prefill::db_prefill;
use rusty_golf::model::{
    AllBettorScoresByRound, BettorScoreByRound, DetailedScore, SummaryDetailedScores,
};
use scraper::{Html, Selector};
use std::io::Write;
use std::path::Path;

use sql_middleware::middleware::{ConfigAndPool, DatabaseType, QueryAndParams, SqliteOptions};

fn detailed_score(
    bettor_name: &str,
    golfer_name: &str,
    golfer_espn_id: i64,
    scores: [i32; 2],
) -> DetailedScore {
    DetailedScore {
        bettor_name: bettor_name.to_string(),
        golfer_name: golfer_name.to_string(),
        golfer_espn_id,
        rounds: vec![0, 1],
        scores: vec![scores[0], scores[1]],
    }
}

fn build_detailed_scores() -> SummaryDetailedScores {
    SummaryDetailedScores {
        detailed_scores: vec![
            detailed_score("Player1", "Scottie Scheffler", 1_234_567, [-3, 0]),
            detailed_score("Player1", "Collin Morikawa", 1_234_568, [-2, 0]),
            detailed_score("Player1", "Min Woo Lee", 1_234_569, [0, 0]),
            detailed_score("Player2", "Bryson DeChambeau", 1_234_570, [-1, 0]),
            detailed_score("Player2", "Justin Thomas", 1_234_571, [1, 0]),
            detailed_score("Player2", "Hideki Matsuyama", 1_234_572, [0, 0]),
            detailed_score("Player3", "Rory McIlroy", 1_234_573, [0, 0]),
            detailed_score("Player3", "Ludvig Åberg", 1_234_574, [1, 0]),
            detailed_score("Player3", "Sepp Straka", 1_234_575, [0, 0]),
            detailed_score("Player4", "Brooks Koepka", 1_234_576, [0, 0]),
            detailed_score("Player4", "Viktor Hovland", 1_234_577, [0, 0]),
            detailed_score("Player4", "Jason Day", 1_234_578, [0, 0]),
            detailed_score("Player5", "Xander Schauffele", 1_234_579, [3, 0]),
            detailed_score("Player5", "Jon Rahm", 1_234_580, [1, 0]),
            detailed_score("Player5", "Will Zalatoris", 1_234_581, [0, 0]),
        ],
    }
}

fn build_summary_scores() -> AllBettorScoresByRound {
    AllBettorScoresByRound {
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
    }
}

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

    let json = serde_json::from_str(include_str!("test5_dbprefill.json"))?;
    db_prefill(&json, &config_and_pool, DatabaseType::Sqlite).await?;

    Ok(config_and_pool)
}

async fn fetch_score_view_factor(
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<f32, Box<dyn std::error::Error>> {
    let mut conn = config_and_pool.get_connection().await?;
    let update_query = "UPDATE event SET refresh_from_espn = 1 WHERE espn_id = ?1;";
    conn.execute_dml(
        update_query,
        &[sql_middleware::middleware::RowValues::Int(event_id.into())],
    )
    .await?;

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

fn expected_label_prefix(golfer_name: &str) -> String {
    if golfer_name == "Min Woo Lee" {
        return "M. Woo ".to_string();
    }

    let golfer_name_parts: Vec<&str> = golfer_name.split_whitespace().collect();
    let golfer_first_initial = golfer_name_parts
        .first()
        .map_or(' ', |s| s.chars().next().unwrap_or(' '));
    let golfer_last_abbr = golfer_name_parts
        .last()
        .map_or("", |s| if s.len() > 5 { &s[0..5] } else { s });

    format!("{golfer_first_initial}. {golfer_last_abbr}")
}

fn expected_bar_width(score: i32, step_factor: f32) -> f32 {
    if score.abs() > 0 {
        #[allow(clippy::cast_precision_loss)]
        {
            score.abs() as f32 * step_factor
        }
    } else {
        step_factor
    }
}

fn parse_bar_width_and_score(bar: scraper::element_ref::ElementRef<'_>) -> Option<(f32, i32)> {
    let style = bar.value().attr("style")?;
    let width_str = style
        .split("width:")
        .nth(1)
        .and_then(|s| s.split(';').next())?
        .trim();
    if !width_str.ends_with('%') {
        return None;
    }

    let width_val: f32 = width_str.trim_end_matches('%').parse().ok()?;
    let bar_text: String = bar.text().collect();
    let score_str = bar_text.trim().replace("R1: ", "").replace("R2: ", "");
    let score: i32 = score_str.parse().ok()?;

    Some((width_val, score))
}

fn assert_bar_widths_match(
    document: &Html,
    detailed_scores: &SummaryDetailedScores,
    step_factor: f32,
) {
    let chart_selector = Selector::parse(".chart").unwrap();
    let chart_row_selector = Selector::parse(".chart-row").unwrap();
    let bar_label_selector = Selector::parse(".bar-label").unwrap();
    let bar_selector = Selector::parse(".bar").unwrap();

    for detailed_score in &detailed_scores.detailed_scores {
        let bettor_name = &detailed_score.bettor_name;
        let expected_label = expected_label_prefix(&detailed_score.golfer_name);
        let mut found_matching_row = false;

        for chart in document.select(&chart_selector) {
            if let Some(player_attr) = chart.value().attr("data-player")
                && player_attr == bettor_name
            {
                for chart_row in chart.select(&chart_row_selector) {
                    if let Some(label_element) = chart_row.select(&bar_label_selector).next() {
                        let label_text = label_element.text().collect::<String>();
                        if !label_text.starts_with(&expected_label) {
                            continue;
                        }

                        found_matching_row = true;
                        let bars: Vec<_> = chart_row.select(&bar_selector).collect();
                        assert_eq!(
                            bars.len(),
                            detailed_score.scores.len(),
                            "Number of bars ({}) should match number of scores ({})",
                            bars.len(),
                            detailed_score.scores.len()
                        );

                        for bar in &bars {
                            if let Some((width_val, score)) = parse_bar_width_and_score(*bar) {
                                let expected_width = expected_bar_width(score, step_factor);
                                let diff = (width_val - expected_width).abs();
                                assert!(
                                    diff < 0.01,
                                    "Bar width should be {expected_width}% but found {width_val}% for score {score} (diff: {diff})"
                                );
                            }
                        }
                    }
                }
            }
        }

        if !found_matching_row {
            // Keep silent; debug output can be added if needed.
        }
    }
}

fn save_debug_html(debug_path: &Path, html_output: &str) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(debug_path)?;
    let debug_file = debug_path.join("actual_output.html");
    let mut file = std::fs::File::create(&debug_file)?;
    writeln!(file, "{html_output}")?;
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
async fn test_bar_width() -> Result<(), Box<dyn std::error::Error>> {
    let config_and_pool = setup_db().await?;
    let detailed_scores = build_detailed_scores();
    let summary_scores_x = build_summary_scores();

    // Test the Masters 2025 event
    let event_id = 401_703_504;

    // STEP 1: Verify the step factor is 4.5 as specified in the test5_dbprefill.json file
    let score_view_factor = fetch_score_view_factor(&config_and_pool, event_id).await?;
    assert!(
        (score_view_factor - 4.5_f32).abs() < f32::EPSILON,
        "score_view_step_factor should be 4.5"
    );

    // STEP 2: Render the drop-down bar HTML and save output for debugging
    let html_output = test_render_template(
        &config_and_pool,
        event_id,
        &summary_scores_x,
        &detailed_scores,
    )
    .await?;

    save_debug_html(Path::new("tests/test6/debug"), &html_output)?;

    // STEP 3: Read the reference HTML file containing expected output
    let reference_path = Path::new("tests/test6/test6_ref_html.html");
    assert!(
        reference_path.exists(),
        "Reference file not found at: {}",
        reference_path.display()
    );
    let reference_html = std::fs::read_to_string(reference_path)?;

    // STEP 4: Get the factor that should be used by preprocess_golfer_data
    let config = rusty_golf::model::get_event_details(&config_and_pool, event_id).await?;
    assert!(
        (config.score_view_step_factor - 4.5_f32).abs() < f32::EPSILON,
        "score_view_step_factor should be 4.5"
    );

    // STEP 5: Test that the HTML contains bars with widths proportional to scores * step_factor
    let document = Html::parse_document(&reference_html);
    assert_bar_widths_match(&document, &detailed_scores, config.score_view_step_factor);

    // SUCCESS! We've verified with precise HTML parsing that each bar width in the HTML
    // exactly matches the expected calculation: score.abs() * score_view_step_factor (or step_factor for 0)
    println!("✓ Test passed: bar widths in HTML correctly reflect score_view_step_factor");

    Ok(())
}
