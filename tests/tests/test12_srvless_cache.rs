mod common;

use chrono::{DateTime, Duration, Utc};
use common::serverless::admin::{admin_cleanup_events, admin_seed_event, admin_update_dates};
use common::serverless::event_id_i32;
use common::serverless::fixtures::{load_espn_cache, load_eup_event, load_score_struct};
use common::serverless::is_local_miniflare;
use common::serverless::locks::{admin_test_lock_retry, admin_test_unlock, test_lock_token};
use common::serverless::runtime::{
    build_local, ensure_command, miniflare_admin_token, miniflare_base_url, run_serverless_enabled,
    wait_for_health, workspace_root, wrangler_paths,
};
use common::serverless::types::AdminSeedRequest;
use serde::Deserialize;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::Path;

#[tokio::test(flavor = "multi_thread")]
async fn test12_serverless_cache_behavior() -> Result<(), Box<dyn Error>> {
    if !run_serverless_enabled() {
        eprintln!("Skipping serverless test: RUN_SERVERLESS=1 not set in .env");
        return Ok(());
    }

    ensure_command("worker-build")?;
    ensure_command("wrangler")?;

    let workspace_root = workspace_root();
    let miniflare_url = miniflare_base_url()?;
    let admin_token = miniflare_admin_token()?;
    let wrangler_paths = wrangler_paths(&workspace_root, "test12");
    let event_id = 401_580_351_i64;
    let lock_token = test_lock_token("test12");

    if is_local_miniflare(&miniflare_url) {
        build_local(&workspace_root, &wrangler_paths)?;
    } else {
        println!("Skipping build_local; MINIFLARE_URL is non-localhost.");
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
        // Force last_refresh into the past so cache age checks are meaningful.
        let last_refresh = (Utc::now() - Duration::days(2)).to_rfc3339();
        // Seed event, golfers, scores, and ESPN cache into KV/R2.
        let payload =
            build_admin_seed_request(&workspace_root, event_id, Some(last_refresh.clone()))?;
        // Initialize serverless storage for the event under test.
        if lock.is_first {
            admin_seed_event(&miniflare_url, &admin_token, &payload).await?;
        }

        // Pull end_date from the cached ESPN header fixture.
        let end_date = load_end_date_from_fixture(&workspace_root, event_id)?;
        // Force end_date into the past; this should no longer make the cache permanent.
        let end_date = normalize_end_date(&end_date)?;
        // Store the past end_date without marking the event completed.
        admin_update_dates(
            &miniflare_url,
            &admin_token,
            event_id,
            None,
            Some(end_date),
            Some(false),
        )
        .await?;

        // First fetch should miss cache because end_date alone is no longer authoritative.
        let cached = fetch_scores_json(event_id, &miniflare_url).await?;
        // Assert JSON reports a cache miss.
        assert_cache_hit(&cached, false)?;

        // Re-seed the same event with completed=true, which should switch to permanent caching.
        let mut completed_payload =
            build_admin_seed_request(&workspace_root, event_id, Some(last_refresh))?;
        completed_payload.event.completed = true;
        if lock.is_first {
            admin_seed_event(&miniflare_url, &admin_token, &completed_payload).await?;
        }

        // Fetch again and verify it now comes from cache because completion is explicit.
        let refreshed = fetch_scores_json(event_id, &miniflare_url).await?;
        // Assert JSON reports a cache hit.
        assert_cache_hit(&refreshed, true)?;

        Ok(())
    }
    .await;

    let is_last = admin_test_unlock(&miniflare_url, &admin_token, event_id, &lock_token).await?;
    if is_last
        && let Err(err) =
            admin_cleanup_events(&miniflare_url, &admin_token, &[event_id], false).await
    {
        eprintln!("admin cleanup failed after test12: {err}");
    }

    test_result
}

fn normalize_end_date(end_date: &str) -> Result<String, Box<dyn Error>> {
    let parsed = DateTime::parse_from_rfc3339(end_date)?;
    let parsed = parsed.with_timezone(&Utc);
    if parsed > Utc::now() {
        Ok((Utc::now() - Duration::days(1)).to_rfc3339())
    } else {
        Ok(parsed.to_rfc3339())
    }
}

async fn fetch_scores_json(event_id: i64, miniflare_url: &str) -> Result<Value, Box<dyn Error>> {
    let resp = reqwest::get(format!(
        "{miniflare_url}/scores?event={event_id}&yr=2024&json=1"
    ))
    .await?;
    assert!(
        resp.status().is_success(),
        "Unexpected status: {}",
        resp.status()
    );
    Ok(resp.json().await?)
}

fn assert_cache_hit(body: &Value, expected: bool) -> Result<(), Box<dyn Error>> {
    let cache_hit = body
        .get("cache_hit")
        .and_then(Value::as_bool)
        .ok_or("Response JSON missing cache_hit")?;
    if cache_hit != expected {
        return Err(format!("Expected cache_hit={expected}, got {cache_hit}").into());
    }
    Ok(())
}

fn load_end_date_from_fixture(
    workspace_root: &Path,
    event_id: i64,
) -> Result<String, Box<dyn Error>> {
    let path = workspace_root.join("tests/tests/test12_espn_header.json");
    let contents = fs::read_to_string(path)?;
    let header: ScoreboardHeader = serde_json::from_str(&contents)?;
    let event_id_str = event_id.to_string();
    for sport in header.sports {
        for league in sport.leagues {
            for event in league.events {
                if event.id == event_id_str
                    && let Some(end_date) = event.end_date
                {
                    return Ok(end_date);
                }
            }
        }
    }
    Err(format!("Missing endDate for event {event_id} in header fixture").into())
}

#[derive(Debug, Deserialize)]
struct ScoreboardHeader {
    sports: Vec<HeaderSport>,
}

#[derive(Debug, Deserialize)]
struct HeaderSport {
    leagues: Vec<HeaderLeague>,
}

#[derive(Debug, Deserialize)]
struct HeaderLeague {
    events: Vec<HeaderEvent>,
}

#[derive(Debug, Deserialize)]
struct HeaderEvent {
    id: String,
    #[serde(rename = "endDate")]
    end_date: Option<String>,
}

fn build_admin_seed_request(
    workspace_root: &Path,
    event_id: i64,
    last_refresh: Option<String>,
) -> Result<AdminSeedRequest, Box<dyn Error>> {
    let event = load_eup_event(workspace_root, event_id)?;
    let score_struct = load_score_struct(workspace_root)?;
    let espn_cache = load_espn_cache(workspace_root)?;
    let event_id = event_id_i32(event_id)?;
    Ok(AdminSeedRequest {
        event_id,
        refresh_from_espn: 1,
        event,
        score_struct,
        espn_cache,
        auth_tokens: None,
        last_refresh,
    })
}
