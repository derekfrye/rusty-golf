mod common;

use common::serverless::admin::{
    admin_cleanup_events, admin_cleanup_scores, admin_scores_exists, admin_seed_event,
    admin_set_espn_failure,
};
use common::serverless::is_local_miniflare;
use common::serverless::locks::{admin_test_lock_retry, admin_test_unlock, test_lock_token};
use common::serverless::runtime::{
    build_admin_seed_request, build_local, ensure_command, miniflare_admin_token,
    miniflare_base_url, run_serverless_enabled, wait_for_health, workspace_root, wrangler_paths,
};
use serde_json::Value;
use std::error::Error;

#[tokio::test(flavor = "multi_thread")]
async fn test13_serverless_espn_failure_falls_back_to_seed_cache() -> Result<(), Box<dyn Error>> {
    if !run_serverless_enabled() {
        eprintln!("Skipping serverless test: RUN_SERVERLESS=1 not set in .env");
        return Ok(());
    }

    ensure_command("worker-build")?;
    ensure_command("wrangler")?;

    let workspace_root = workspace_root();
    let miniflare_url = miniflare_base_url()?;
    let admin_token = miniflare_admin_token()?;
    let wrangler_paths = wrangler_paths(&workspace_root, "test13");
    let event_id = 401_580_351_i64;
    let lock_token = test_lock_token("test13");

    if is_local_miniflare(&miniflare_url) {
        println!("Skipping build_local; MINIFLARE_URL is localhost.");
    } else {
        build_local(&workspace_root, &wrangler_paths)?;
    }
    wait_for_health(&format!("{miniflare_url}/health")).await?;
    println!("miniflare health check passed");

    let lock = admin_test_lock_retry(
        &miniflare_url,
        &admin_token,
        event_id,
        &lock_token,
        "exclusive",
    )
    .await?;
    if lock.is_first {
        admin_cleanup_events(&miniflare_url, &admin_token, &[event_id], false).await?;
    }

    let test_result = async {
        if lock.is_first {
            let payload = build_admin_seed_request(&workspace_root, event_id, None)?;
            admin_seed_event(&miniflare_url, &admin_token, &payload).await?;
        }

        admin_cleanup_scores(&miniflare_url, &admin_token, event_id).await?;
        let (scores_exists, espn_cache_exists) =
            admin_scores_exists(&miniflare_url, &admin_token, event_id).await?;
        assert!(!scores_exists, "Expected scores.json to be deleted");
        assert!(espn_cache_exists, "Expected espn_cache to exist");
        admin_set_espn_failure(&miniflare_url, &admin_token, event_id, true).await?;

        assert_scores_response(event_id, &miniflare_url).await?;
        let (scores_exists, _) =
            admin_scores_exists(&miniflare_url, &admin_token, event_id).await?;
        assert!(scores_exists, "Expected scores.json to be restored");

        Ok(())
    }
    .await;

    let _ = admin_set_espn_failure(&miniflare_url, &admin_token, event_id, false).await;
    let is_last = admin_test_unlock(&miniflare_url, &admin_token, event_id, &lock_token).await?;
    if is_last
        && let Err(err) =
            admin_cleanup_events(&miniflare_url, &admin_token, &[event_id], false).await
    {
        eprintln!("admin cleanup failed after test13: {err}");
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
