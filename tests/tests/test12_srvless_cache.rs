mod common;

use chrono::{DateTime, Duration, Utc};
use common::serverless::{
    AdminSeedRequest, WranglerPaths, admin_cleanup_events, admin_seed_event, admin_test_lock_retry,
    admin_test_unlock, admin_update_end_date, event_id_i32, is_local_miniflare, load_espn_cache,
    load_eup_event, load_score_struct, shared_wrangler_dirs, test_lock_token,
};
use serde::Deserialize;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration as StdDuration;

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
    let wrangler_paths = wrangler_paths(&workspace_root);
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
        let payload = build_admin_seed_request(&workspace_root, event_id, Some(last_refresh))?;
        // Initialize serverless storage for the event under test.
        if lock.is_first {
            admin_seed_event(&miniflare_url, &admin_token, &payload).await?;
        }

        // Pull end_date from the cached ESPN header fixture.
        let end_date = load_end_date_from_fixture(&workspace_root, event_id)?;
        // Ensure end_date is in the past to enable permanent cache behavior.
        let end_date = normalize_end_date(&end_date)?;
        // Store the past end_date in KV so cache is treated as permanent.
        admin_update_end_date(&miniflare_url, &admin_token, event_id, Some(end_date)).await?;

        // First fetch should hit cache because end_date is in the past.
        let cached = fetch_scores_json(event_id, &miniflare_url).await?;
        // Assert JSON reports a cache hit.
        assert_cache_hit(&cached, true)?;

        // Move end_date into the future to disable permanent caching.
        let tomorrow = (Utc::now() + Duration::days(1)).to_rfc3339();
        // Persist the future end_date so cache is no longer authoritative.
        admin_update_end_date(&miniflare_url, &admin_token, event_id, Some(tomorrow)).await?;

        // Fetch again and verify it does not come from cache.
        let refreshed = fetch_scores_json(event_id, &miniflare_url).await?;
        // Assert JSON reports a cache miss.
        assert_cache_hit(&refreshed, false)?;

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

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
}

fn run_serverless_enabled() -> bool {
    init_env();
    std::env::var("RUN_SERVERLESS")
        .map(|value| value.trim() == "1")
        .unwrap_or(false)
}

fn ensure_command(cmd: &str) -> Result<(), Box<dyn Error>> {
    let status = Command::new("which").arg(cmd).status()?;
    if !status.success() {
        return Err(format!("Required command not found: {cmd}").into());
    }
    Ok(())
}

fn run_script(script_path: &Path, envs: &[(&str, &str)], cwd: &Path) -> Result<(), Box<dyn Error>> {
    let output = Command::new("bash")
        .arg(script_path)
        .envs(envs.iter().copied())
        .current_dir(cwd)
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "Script failed: {}\nstdout:\n{}\nstderr:\n{}",
        script_path.display(),
        stdout,
        stderr
    )
    .into())
}

fn wrangler_paths(workspace_root: &Path) -> WranglerPaths {
    let (log_dir, config_dir) = shared_wrangler_dirs().unwrap_or_else(|| {
        (
            workspace_root.join(".wrangler-logs-test12"),
            workspace_root.join(".wrangler-config-test12"),
        )
    });
    WranglerPaths {
        config: workspace_root.join("serverless/wrangler.toml"),
        log_dir,
        config_dir,
    }
}

fn build_local(
    workspace_root: &Path,
    wrangler_paths: &WranglerPaths,
) -> Result<(), Box<dyn Error>> {
    println!(
        "Using wrangler config: {}, log dir: {}",
        wrangler_paths.config.display(),
        wrangler_paths.log_dir.display()
    );
    let now = chrono::Local::now();
    println!("Starting build_local at {}", now.format("%H:%M:%S"));
    let wrangler_log_dir_str = wrangler_paths.log_dir.to_str().unwrap_or_default();
    let wrangler_config_dir_str = wrangler_paths.config_dir.to_str().unwrap_or_default();
    run_script(
        &workspace_root.join("serverless/scripts/build_local.sh"),
        &[
            (
                "CONFIG_PATH",
                wrangler_paths.config.to_str().unwrap_or_default(),
            ),
            ("WRANGLER_LOG_DIR", wrangler_log_dir_str),
            ("XDG_CONFIG_HOME", wrangler_config_dir_str),
        ],
        workspace_root,
    )?;
    let now = chrono::Local::now();
    println!("build_local completed at {}", now.format("%H:%M:%S"));
    Ok(())
}

async fn wait_for_health(url: &str) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(2))
        .build()?;
    for _ in 0..240 {
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            _ => tokio::time::sleep(StdDuration::from_millis(250)).await,
        }
    }
    Err(format!("Timed out waiting for {url}").into())
}

fn init_env() {
    let _ = dotenvy::dotenv();
    if std::env::var("MINIFLARE_URL").is_err() || std::env::var("MINIFLARE_ADMIN_TOKEN").is_err() {
        let _ = dotenvy::from_filename("../.env");
    }
}

fn miniflare_base_url() -> Result<String, Box<dyn Error>> {
    init_env();
    let url = std::env::var("MINIFLARE_URL")
        .map_err(|_| "MINIFLARE_URL not set in ../.env or environment")?;
    Ok(url.trim_end_matches('/').to_string())
}

fn miniflare_admin_token() -> Result<String, Box<dyn Error>> {
    init_env();
    let token = std::env::var("MINIFLARE_ADMIN_TOKEN")
        .map_err(|_| "MINIFLARE_ADMIN_TOKEN not set in ../.env or environment")?;
    Ok(token)
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
