mod common;

use common::serverless::{
    AdminSeedRequest, WranglerPaths, admin_cleanup_events, admin_seed_event, admin_test_lock_retry,
    admin_test_unlock, event_id_i32, is_local_miniflare, load_espn_cache, load_eup_event,
    load_score_struct, shared_wrangler_dirs, test_lock_token,
};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

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
    let wrangler_paths = wrangler_paths(&workspace_root);
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
            workspace_root.join(".wrangler-logs-test11"),
            workspace_root.join(".wrangler-config-test11"),
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
    )
}

async fn wait_for_health(url: &str) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;
    for _ in 0..240 {
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            _ => tokio::time::sleep(Duration::from_millis(250)).await,
        }
    }
    Err(format!("Timed out waiting for {url}").into())
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
    auth_tokens: Option<Vec<String>>,
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
        auth_tokens,
        last_refresh: None,
    })
}
