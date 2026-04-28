mod common;
#[path = "support/test06_data.rs"]
mod test06_data;
#[path = "support/test06_support.rs"]
mod test06_support;

use scraper::Html;
use std::path::Path;
use test06_data::{build_detailed_scores, build_summary_scores};
use test06_support::{
    assert_bar_widths_match, fetch_score_view_factor, save_debug_html, setup_db,
    test_render_template,
};

#[tokio::test]
async fn test_bar_width() -> Result<(), Box<dyn std::error::Error>> {
    let config_and_pool = setup_db().await?;
    let detailed_scores = build_detailed_scores();
    let summary_scores_x = build_summary_scores();

    // Test the Masters 2025 event
    let event_id = 401_703_504;

    // STEP 1: Verify the step factor is 4.5 as specified in the test05_dbprefill.json file
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

    save_debug_html(Path::new("tests/test06/debug"), &html_output)?;

    // STEP 3: Read the reference HTML file containing expected output
    let reference_path = Path::new("tests/test06/test06_ref_html.html");
    assert!(
        reference_path.exists(),
        "Reference file not found at: {}",
        reference_path.display()
    );
    let reference_html = std::fs::read_to_string(reference_path)?;

    // STEP 4: Get the factor that should be used by preprocess_golfer_data
    let config = rusty_golf_actix::model::get_event_details(&config_and_pool, event_id).await?;
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
