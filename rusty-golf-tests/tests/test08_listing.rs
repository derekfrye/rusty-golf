mod common;

use common::serverless::{CleanupGuard, WranglerFlags, WranglerPaths, cleanup_events};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use rusty_golf_setup::{SeedOptions, seed_kv_from_eup};

#[tokio::test(flavor = "multi_thread")]
async fn test08_listing_endpoint() -> Result<(), Box<dyn Error>> {
    if !run_serverless_enabled() {
        eprintln!("Skipping serverless test: RUN_SERVERLESS=1 not set in .env");
        return Ok(());
    }

    ensure_command("worker-build")?;
    ensure_command("wrangler")?;
    ensure_command("jq")?;

    let workspace_root = workspace_root();
    let miniflare_url = miniflare_base_url()?;
    let wrangler_paths = wrangler_paths(&workspace_root);
    let wrangler_flags = wrangler_flags(&wrangler_paths.config);

    let event_ids = vec![401_703_504_i64, 401_703_521_i64];
    cleanup_events(
        &workspace_root,
        &wrangler_paths,
        &wrangler_flags,
        &event_ids,
        true,
    )?;
    let _cleanup_guard = CleanupGuard::new(
        workspace_root.clone(),
        wrangler_paths.clone(),
        wrangler_flags.clone(),
        event_ids.clone(),
        true,
    );

    build_local(&workspace_root, &wrangler_paths)?;

    let auth_token = "listing-token-123";
    let auth_tokens = vec![auth_token.to_string()];
    seed_listing_kv(
        &workspace_root,
        &wrangler_paths,
        &wrangler_flags,
        &auth_tokens,
    )?;

    seed_listing_r2(&workspace_root, &wrangler_paths, &wrangler_flags)?;
    wait_for_health(&format!("{}/health", miniflare_url)).await?;

    assert_listing_response(auth_token, &miniflare_url).await?;

    Ok(())
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
    WranglerPaths {
        config: workspace_root.join("rusty-golf-serverless/wrangler.toml"),
        log_dir: workspace_root.join(".wrangler-logs-listing"),
        config_dir: workspace_root.join(".wrangler-config-listing"),
    }
}

fn wrangler_flags(config: &Path) -> WranglerFlags {
    let kv_flags = format!(
        "--local --preview false --config {} --env dev",
        config.display()
    );
    let r2_flags = format!("--local --config {} --env dev", config.display());
    let kv_flag_list = kv_flags
        .split_whitespace()
        .map(ToString::to_string)
        .collect();
    WranglerFlags {
        kv_flags,
        r2_flags,
        kv_flag_list,
    }
}

fn build_local(
    workspace_root: &Path,
    wrangler_paths: &WranglerPaths,
) -> Result<(), Box<dyn Error>> {
    let wrangler_log_dir_str = wrangler_paths.log_dir.to_str().unwrap_or_default();
    let wrangler_config_dir_str = wrangler_paths.config_dir.to_str().unwrap_or_default();
    run_script(
        &workspace_root.join("rusty-golf-serverless/scripts/build_local.sh"),
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

fn seed_listing_kv(
    workspace_root: &Path,
    wrangler_paths: &WranglerPaths,
    wrangler_flags: &WranglerFlags,
    auth_tokens: &[String],
) -> Result<(), Box<dyn Error>> {
    for event_id in [401_703_504_i64, 401_703_521_i64] {
        seed_kv_from_eup(&SeedOptions {
            eup_json: workspace_root.join("rusty-golf-tests/tests/test05_dbprefill.json"),
            kv_env: "dev".to_string(),
            kv_binding: Some("djf_rusty_golf_kv".to_string()),
            auth_tokens: Some(auth_tokens.to_vec()),
            event_id: Some(event_id),
            refresh_from_espn: 1,
            wrangler_config: wrangler_paths.config.clone(),
            wrangler_env: "dev".to_string(),
            wrangler_kv_flags: wrangler_flags.kv_flag_list.clone(),
            wrangler_log_dir: Some(wrangler_paths.log_dir.clone()),
            wrangler_config_dir: Some(wrangler_paths.config_dir.clone()),
        })?;
    }
    Ok(())
}

fn seed_listing_r2(
    workspace_root: &Path,
    wrangler_paths: &WranglerPaths,
    wrangler_flags: &WranglerFlags,
) -> Result<(), Box<dyn Error>> {
    let wrangler_log_dir_str = wrangler_paths.log_dir.to_str().unwrap_or_default();
    let wrangler_config_dir_str = wrangler_paths.config_dir.to_str().unwrap_or_default();
    run_script(
        &workspace_root.join("rusty-golf-serverless/scripts/seed_test1_local.sh"),
        &[
            ("WRANGLER_KV_FLAGS", wrangler_flags.kv_flags.as_str()),
            ("WRANGLER_R2_FLAGS", wrangler_flags.r2_flags.as_str()),
            // wrangler dev --local reads from the preview R2 bucket by default.
            ("R2_BUCKET", "djf-rusty-golf-dev-preview"),
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
    let resp =
        reqwest::get(format!("{miniflare_url}/listing?auth_token={auth_token}")).await?;
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
        body.contains("401703521"),
        "Listing missing event 401703521"
    );
    assert!(
        body.contains("The Open 2025"),
        "Listing missing The Open 2025"
    );
    Ok(())
}

fn init_env() {
    let _ = dotenvy::dotenv();
    if std::env::var("MINIFLARE_URL").is_err() {
        let _ = dotenvy::from_filename("../.env");
    }
}

fn miniflare_base_url() -> Result<String, Box<dyn Error>> {
    init_env();
    let url = std::env::var("MINIFLARE_URL")
        .map_err(|_| "MINIFLARE_URL not set in ../.env or environment")?;
    Ok(url.trim_end_matches('/').to_string())
}
