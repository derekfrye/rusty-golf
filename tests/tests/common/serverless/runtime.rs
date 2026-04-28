use super::fixtures::{load_espn_cache, load_eup_event, load_score_struct};
use super::types::{AdminSeedRequest, WranglerPaths};
use super::{event_id_i32, shared_wrangler_dirs};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
}

pub fn run_serverless_enabled() -> bool {
    init_env();
    std::env::var("RUN_SERVERLESS").is_ok_and(|value| value.trim() == "1")
}

pub fn ensure_command(cmd: &str) -> Result<(), Box<dyn Error>> {
    let status = Command::new("which").arg(cmd).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Required command not found: {cmd}").into())
    }
}

pub fn build_local(
    workspace_root: &Path,
    wrangler_paths: &WranglerPaths,
) -> Result<(), Box<dyn Error>> {
    run_script(
        &workspace_root.join("serverless/scripts/build_local.sh"),
        &[
            (
                "CONFIG_PATH",
                wrangler_paths.config.to_str().unwrap_or_default(),
            ),
            (
                "WRANGLER_LOG_DIR",
                wrangler_paths.log_dir.to_str().unwrap_or_default(),
            ),
            (
                "XDG_CONFIG_HOME",
                wrangler_paths.config_dir.to_str().unwrap_or_default(),
            ),
        ],
        workspace_root,
    )
}

pub async fn wait_for_health(url: &str) -> Result<(), Box<dyn Error>> {
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

pub fn miniflare_base_url() -> Result<String, Box<dyn Error>> {
    init_env();
    let url = std::env::var("MINIFLARE_URL")
        .map_err(|_| "MINIFLARE_URL not set in ../.env or environment")?;
    Ok(url.trim_end_matches('/').to_string())
}

pub fn miniflare_admin_token() -> Result<String, Box<dyn Error>> {
    init_env();
    std::env::var("MINIFLARE_ADMIN_TOKEN")
        .map_err(|_| "MINIFLARE_ADMIN_TOKEN not set in ../.env or environment".into())
}

pub fn build_admin_seed_request(
    workspace_root: &Path,
    event_id: i64,
    auth_tokens: Option<Vec<String>>,
) -> Result<AdminSeedRequest, Box<dyn Error>> {
    Ok(AdminSeedRequest {
        event_id: event_id_i32(event_id)?,
        refresh_from_espn: 1,
        event: load_eup_event(workspace_root, event_id)?,
        score_struct: load_score_struct(workspace_root)?,
        espn_cache: load_espn_cache(workspace_root)?,
        auth_tokens,
        last_refresh: None,
    })
}

pub fn wrangler_paths(workspace_root: &Path, test_name: &str) -> WranglerPaths {
    let (log_dir, config_dir) = shared_wrangler_dirs().unwrap_or_else(|| {
        (
            workspace_root.join(format!(".wrangler-logs-{test_name}")),
            workspace_root.join(format!(".wrangler-config-{test_name}")),
        )
    });
    WranglerPaths {
        config: workspace_root.join("serverless/wrangler.toml"),
        log_dir,
        config_dir,
    }
}

fn init_env() {
    let _ = dotenvy::dotenv();
    if std::env::var("MINIFLARE_URL").is_err() || std::env::var("MINIFLARE_ADMIN_TOKEN").is_err() {
        let _ = dotenvy::from_filename("../.env");
    }
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
