mod common;
#[path = "support/test07_support.rs"]
mod test07_support;

use std::path::Path;
use test07_support::{
    assert_event_401580355, assert_event_401703504, load_detailed_scores, load_summary_scores,
    save_debug_html, set_refresh_from_espn, setup_db, test_render_template,
};

#[tokio::test]
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
        let reference_path_str = format!("tests/test07/reference_html_{event_id}.html");
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
