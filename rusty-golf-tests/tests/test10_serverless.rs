mod common;

use common::serverless::{
    AdminCleanupGuard,
    AdminSeedRequest,
    WranglerPaths,
    admin_cleanup_events,
    admin_seed_event,
    load_espn_cache,
    load_eup_event,
    load_score_struct,
    shared_wrangler_dirs,
};
use serde_json::Value;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

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
    let wrangler_paths = wrangler_paths(&workspace_root);
    let event_id = 401_580_351_i64;

    build_local(&workspace_root, &wrangler_paths)?;
    wait_for_health(&format!("{}/health", miniflare_url)).await?;
    println!("miniflare health check passed");

    admin_cleanup_events(&miniflare_url, &admin_token, &[event_id], false).await?;
    let _cleanup_guard =
        AdminCleanupGuard::new(miniflare_url.clone(), admin_token.clone(), vec![event_id], false);

    let payload = build_admin_seed_request(&workspace_root, event_id, None)?;
    admin_seed_event(&miniflare_url, &admin_token, &payload).await?;

    assert_scores_response(event_id, &miniflare_url).await?;

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
    let (log_dir, config_dir) = shared_wrangler_dirs().unwrap_or_else(|| {
        (
            workspace_root.join(".wrangler-logs-test10"),
            workspace_root.join(".wrangler-config-test10"),
        )
    });
    WranglerPaths {
        config: workspace_root.join("rusty-golf-serverless/wrangler.toml"),
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
    )?;
    let now = chrono::Local::now();
    println!("build_local completed at {}", now.format("%H:%M:%S"));
    Ok(())
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

async fn assert_scores_response(
    event_id: i64,
    miniflare_url: &str,
) -> Result<(), Box<dyn Error>> {
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

    let reference_result: Value = serde_json::from_str(include_str!("test01_expected_output.json"))?;
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

fn init_env() {
    let _ = dotenvy::dotenv();
    if std::env::var("MINIFLARE_URL").is_err()
        || std::env::var("MINIFLARE_ADMIN_TOKEN").is_err()
    {
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
    Ok(AdminSeedRequest {
        event_id: event_id as i32,
        refresh_from_espn: 1,
        event,
        score_struct,
        espn_cache,
        auth_tokens,
        last_refresh: Some("2024-05-19T00:00:00Z".to_string()),
    })
}
