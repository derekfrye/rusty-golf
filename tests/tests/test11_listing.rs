mod common;

use common::serverless::admin::{admin_cleanup_events, admin_seed_event};
use common::serverless::is_local_miniflare;
use common::serverless::locks::{admin_test_lock_retry, admin_test_unlock, test_lock_token};
use common::serverless::runtime::{
    build_admin_seed_request, build_local, ensure_command, miniflare_admin_token,
    miniflare_base_url, run_serverless_enabled, wait_for_health, workspace_root, wrangler_paths,
};
use std::error::Error;

#[tokio::test(flavor = "multi_thread")]
async fn test11_listing_endpoint() -> Result<(), Box<dyn Error>> {
    if !run_serverless_enabled() {
        eprintln!("Skipping serverless test: RUN_SERVERLESS=1 not set in .env");
        return Ok(());
    }

    ensure_command("worker-build")?;
    ensure_command("wrangler")?;

    let workspace_root = workspace_root();
    let miniflare_url = miniflare_base_url()?;
    let admin_token = miniflare_admin_token()?;
    let wrangler_paths = wrangler_paths(&workspace_root, "test11");
    let lock_token = test_lock_token("test11");

    // Use event IDs that do not overlap with test10 so nextest can run in parallel.
    let event_ids = vec![401_703_504_i64, 401_580_360_i64];
    if is_local_miniflare(&miniflare_url) {
        build_local(&workspace_root, &wrangler_paths)?;
    } else {
        println!("Skipping build_local; MINIFLARE_URL is non-localhost.");
    }

    let auth_token = "listing-token-123";
    let auth_tokens = vec![auth_token.to_string()];
    wait_for_health(&format!("{miniflare_url}/health")).await?;

    let mut first_events = Vec::new();
    for event_id in &event_ids {
        let lock = admin_test_lock_retry(
            &miniflare_url,
            &admin_token,
            *event_id,
            &lock_token,
            "shared",
        )
        .await?;
        if lock.is_first {
            first_events.push(*event_id);
        }
    }
    if !first_events.is_empty() {
        admin_cleanup_events(&miniflare_url, &admin_token, &first_events, true).await?;
    }

    let test_result = async {
        for event_id in &first_events {
            let payload =
                build_admin_seed_request(&workspace_root, *event_id, Some(auth_tokens.clone()))?;
            admin_seed_event(&miniflare_url, &admin_token, &payload).await?;
        }

        assert_listing_response(auth_token, &miniflare_url).await?;

        Ok(())
    }
    .await;

    let mut last_events = Vec::new();
    for event_id in &event_ids {
        let is_last =
            admin_test_unlock(&miniflare_url, &admin_token, *event_id, &lock_token).await?;
        if is_last {
            last_events.push(*event_id);
        }
    }
    if !last_events.is_empty()
        && let Err(err) =
            admin_cleanup_events(&miniflare_url, &admin_token, &last_events, true).await
    {
        eprintln!("admin cleanup failed after test11: {err}");
    }

    test_result
}

async fn assert_listing_response(
    auth_token: &str,
    miniflare_url: &str,
) -> Result<(), Box<dyn Error>> {
    let resp = reqwest::get(format!("{miniflare_url}/listing?auth_token={auth_token}")).await?;
    assert!(
        resp.status().is_success(),
        "Unexpected status: {}",
        resp.status()
    );
    let body = resp.text().await?;
    assert!(body.contains("<table>"), "Listing HTML missing table");
    assert!(
        body.contains("401703504"),
        "Listing missing event 401703504"
    );
    assert!(
        body.contains("Masters Tournament 2025"),
        "Listing missing Masters Tournament 2025"
    );
    assert!(
        body.contains("401580360"),
        "Listing missing event 401580360"
    );
    assert!(body.contains("The Open"), "Listing missing The Open");
    Ok(())
}
