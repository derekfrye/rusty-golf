mod common;

use common::serverless::admin::{admin_cleanup_events, admin_seed_event};
use common::serverless::is_local_miniflare;
use common::serverless::locks::{admin_test_lock_retry, admin_test_unlock, test_lock_token};
use common::serverless::runtime::{
    build_admin_seed_request, build_local, ensure_command, miniflare_admin_token,
    miniflare_base_url, run_serverless_enabled, wait_for_health, workspace_root, wrangler_paths,
};
use serde_json::Value;
use std::error::Error;

#[tokio::test(flavor = "multi_thread")]
async fn test10_serverless_scores_endpoint() -> Result<(), Box<dyn Error>> {
    if !run_serverless_enabled() {
        eprintln!("Skipping serverless test: RUN_SERVERLESS=1 not set in .env");
        return Ok(());
    }

    ensure_command("worker-build")?;
    ensure_command("wrangler")?;

    let workspace_root = workspace_root();
    let miniflare_url = miniflare_base_url()?;
    let admin_token = miniflare_admin_token()?;
    let wrangler_paths = wrangler_paths(&workspace_root, "test10");
    let event_id = 401_580_351_i64;
    let lock_token = test_lock_token("test10");

    if is_local_miniflare(&miniflare_url) {
        build_local(&workspace_root, &wrangler_paths)?;
    } else {
        println!("Skipping build_local; MINIFLARE_URL is non-localhost.");
    }
    wait_for_health(&format!("{miniflare_url}/health")).await?;
    println!("miniflare health check passed");

    // Use an exclusive lock because test12 mutates the same event during parallel runs
    // (e.g., toggling end_date and forcing refresh), which can overwrite fixture scores.
    let _lock = admin_test_lock_retry(
        &miniflare_url,
        &admin_token,
        event_id,
        &lock_token,
        "exclusive",
    )
    .await?;
    admin_cleanup_events(&miniflare_url, &admin_token, &[event_id], false).await?;

    let test_result = async {
        let payload = build_admin_seed_request(&workspace_root, event_id, None)?;
        admin_seed_event(&miniflare_url, &admin_token, &payload).await?;

        assert_scores_response(event_id, &miniflare_url).await?;

        Ok(())
    }
    .await;

    let is_last = admin_test_unlock(&miniflare_url, &admin_token, event_id, &lock_token).await?;
    if is_last
        && let Err(err) =
            admin_cleanup_events(&miniflare_url, &admin_token, &[event_id], false).await
    {
        eprintln!("admin cleanup failed after test10: {err}");
    }

    test_result
}

async fn assert_scores_response(event_id: i64, miniflare_url: &str) -> Result<(), Box<dyn Error>> {
    let resp = reqwest::get(format!(
        "{miniflare_url}/scores?event={event_id}&yr=2024&cache=1&json=1"
    ))
    .await?;
    println!("Received response from /scores endpoint");
    assert!(
        resp.status().is_success(),
        "Unexpected status: {}",
        resp.status()
    );
    let body: Value = resp.json().await?;
    assert!(
        body.is_object(),
        "Response is not a JSON object; got {body:?}"
    );

    let bettor_struct = body
        .get("bettor_struct")
        .and_then(|v| v.as_array())
        .expect("Response JSON does not contain 'bettor_struct' array");
    assert_eq!(
        bettor_struct.len(),
        5,
        "Unexpected number of bettors returned"
    );

    let reference_result: Value =
        serde_json::from_str(include_str!("test01_expected_output.json"))?;
    let reference_array = reference_result
        .get("bettor_struct")
        .and_then(|v| v.as_array())
        .expect("Reference JSON missing bettor_struct");

    for bettor in bettor_struct {
        let bettor_name = bettor
            .get("bettor_name")
            .and_then(Value::as_str)
            .expect("Score entry missing 'bettor_name'");
        let total_score = bettor
            .get("total_score")
            .and_then(Value::as_i64)
            .expect("Score entry missing 'total_score'");
        let scoreboard_position = bettor
            .get("scoreboard_position")
            .and_then(Value::as_i64)
            .expect("Score entry missing 'scoreboard_position'");
        let scoreboard_position_name = bettor
            .get("scoreboard_position_name")
            .and_then(Value::as_str)
            .expect("Score entry missing 'scoreboard_position_name'");

        let reference_bettor = reference_array
            .iter()
            .find(|candidate| {
                candidate.get("bettor_name").and_then(Value::as_str) == Some(bettor_name)
            })
            .unwrap_or_else(|| panic!("Reference JSON missing bettor '{bettor_name}'"));

        assert_eq!(
            total_score,
            reference_bettor
                .get("total_score")
                .and_then(Value::as_i64)
                .expect("Reference entry missing total_score"),
            "Total score mismatch for bettor '{bettor_name}'"
        );

        assert_eq!(
            scoreboard_position,
            reference_bettor
                .get("scoreboard_position")
                .and_then(Value::as_i64)
                .expect("Reference entry missing scoreboard_position"),
            "Scoreboard position mismatch for bettor '{bettor_name}'"
        );

        assert_eq!(
            scoreboard_position_name,
            reference_bettor
                .get("scoreboard_position_name")
                .and_then(Value::as_str)
                .expect("Reference entry missing scoreboard_position_name"),
            "Scoreboard position name mismatch for bettor '{bettor_name}'"
        );
    }

    Ok(())
}
