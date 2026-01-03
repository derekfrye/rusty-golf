use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
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
    let wrangler_paths = wrangler_paths(&workspace_root);
    let wrangler_flags = wrangler_flags(&wrangler_paths.config);
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
    let (guard, log_paths) = start_miniflare(&workspace_root, &wrangler_paths, "8788")?;
    let _guard = guard;

    if let Err(err) = wait_for_health("http://127.0.0.1:8788/health").await {
        let (stdout, stderr) = read_wrangler_logs(&log_paths);
        return Err(format!(
            "{err}\nwrangler dev stdout:\n{stdout}\nwrangler dev stderr:\n{stderr}"
        )
        .into());
    }

    assert_listing_response(auth_token).await?;

    Ok(())
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
}

fn run_serverless_enabled() -> bool {
    let env_path = workspace_root().join(".env");
    let Ok(contents) = std::fs::read_to_string(env_path) else {
        return false;
    };
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=')
            && key.trim() == "RUN_SERVERLESS"
            && value.trim() == "1"
        {
            return true;
        }
    }
    false
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

struct WranglerPaths {
    config: PathBuf,
    log_dir: PathBuf,
    config_dir: PathBuf,
}

struct WranglerFlags {
    kv_flags: String,
    r2_flags: String,
    kv_flag_list: Vec<String>,
}

struct WranglerLogPaths {
    stdout_path: PathBuf,
    stderr_path: PathBuf,
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
            eup_json: workspace_root.join("rusty-golf-tests/tests/test5_dbprefill.json"),
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

fn start_miniflare(
    workspace_root: &Path,
    wrangler_paths: &WranglerPaths,
    port: &str,
) -> Result<(ChildGuard, WranglerLogPaths), Box<dyn Error>> {
    let stdout_path = wrangler_paths.log_dir.join("wrangler-dev-stdout.log");
    let stderr_path = wrangler_paths.log_dir.join("wrangler-dev-stderr.log");
    let stdout_file = std::fs::File::create(&stdout_path)?;
    let stderr_file = std::fs::File::create(&stderr_path)?;
    let wrangler_log_dir_str = wrangler_paths.log_dir.to_str().unwrap_or_default();
    let wrangler_config_dir_str = wrangler_paths.config_dir.to_str().unwrap_or_default();

    let child = Command::new("bash")
        .arg(workspace_root.join("rusty-golf-serverless/scripts/start_miniflare_local.sh"))
        .arg(port)
        .arg(&wrangler_paths.config)
        .current_dir(workspace_root)
        .env("WRANGLER_LOG_DIR", wrangler_log_dir_str)
        .env("XDG_CONFIG_HOME", wrangler_config_dir_str)
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()?;

    Ok((
        ChildGuard { child },
        WranglerLogPaths {
            stdout_path,
            stderr_path,
        },
    ))
}

fn read_wrangler_logs(paths: &WranglerLogPaths) -> (String, String) {
    let stdout = std::fs::read_to_string(&paths.stdout_path).unwrap_or_default();
    let stderr = std::fs::read_to_string(&paths.stderr_path).unwrap_or_default();
    (stdout, stderr)
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

async fn assert_listing_response(auth_token: &str) -> Result<(), Box<dyn Error>> {
    let resp = reqwest::get(format!(
        "http://127.0.0.1:8788/listing?auth_token={auth_token}"
    ))
    .await?;
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

struct ChildGuard {
    child: Child,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
