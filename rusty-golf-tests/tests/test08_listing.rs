use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use rusty_golf_setup::{seed_kv_from_eup, SeedOptions};

#[tokio::test(flavor = "multi_thread")]
async fn test08_listing_endpoint() -> Result<(), Box<dyn Error>> {
    if !run_serverless_enabled()? {
        eprintln!("Skipping serverless test: RUN_SERVERLESS=1 not set in .env");
        return Ok(());
    }

    ensure_command("worker-build")?;
    ensure_command("wrangler")?;
    ensure_command("jq")?;

    let workspace_root = workspace_root();
    let wrangler_config = workspace_root.join("rusty-golf-serverless/wrangler.toml");
    let wrangler_log_dir = workspace_root.join(".wrangler-logs-listing");
    let wrangler_log_dir_str = wrangler_log_dir.to_str().unwrap_or_default();
    let wrangler_config_dir = workspace_root.join(".wrangler-config-listing");
    let wrangler_config_dir_str = wrangler_config_dir.to_str().unwrap_or_default();

    run_script(
        &workspace_root.join("rusty-golf-serverless/scripts/build_local.sh"),
        &[
            ("CONFIG_PATH", wrangler_config.to_str().unwrap_or_default()),
            ("WRANGLER_LOG_DIR", wrangler_log_dir_str),
            ("XDG_CONFIG_HOME", wrangler_config_dir_str),
        ],
        &workspace_root,
    )?;

    let wrangler_kv_flags = format!(
        "--local --preview false --config {} --env dev",
        wrangler_config.display()
    );
    let wrangler_r2_flags = format!("--local --config {} --env dev", wrangler_config.display());

    let auth_token = "listing-token-123";
    let auth_tokens = vec![auth_token.to_string()];
    let wrangler_kv_flag_list = wrangler_kv_flags
        .split_whitespace()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    for event_id in [401_703_504_i64, 401_703_521_i64] {
        seed_kv_from_eup(&SeedOptions {
            eup_json: workspace_root.join("rusty-golf-tests/tests/test5_dbprefill.json"),
            kv_env: "dev".to_string(),
            kv_binding: Some("djf_rusty_golf_kv".to_string()),
            auth_tokens: Some(auth_tokens.clone()),
            event_id: Some(event_id),
            refresh_from_espn: 1,
            wrangler_config: wrangler_config.clone(),
            wrangler_env: "dev".to_string(),
            wrangler_kv_flags: wrangler_kv_flag_list.clone(),
            wrangler_log_dir: Some(wrangler_log_dir.clone()),
            wrangler_config_dir: Some(wrangler_config_dir.clone()),
        })?;
    }

    run_script(
        &workspace_root.join("rusty-golf-serverless/scripts/seed_test1_local.sh"),
        &[
            ("WRANGLER_KV_FLAGS", wrangler_kv_flags.as_str()),
            ("WRANGLER_R2_FLAGS", wrangler_r2_flags.as_str()),
            // wrangler dev --local reads from the preview R2 bucket by default.
            ("R2_BUCKET", "djf-rusty-golf-dev-preview"),
            ("WRANGLER_LOG_DIR", wrangler_log_dir_str),
            ("XDG_CONFIG_HOME", wrangler_config_dir_str),
        ],
        &workspace_root,
    )?;

    let stdout_path = wrangler_log_dir.join("wrangler-dev-stdout.log");
    let stderr_path = wrangler_log_dir.join("wrangler-dev-stderr.log");
    let stdout_file = std::fs::File::create(&stdout_path)?;
    let stderr_file = std::fs::File::create(&stderr_path)?;

    let child = Command::new("bash")
        .arg(
            workspace_root.join("rusty-golf-serverless/scripts/start_miniflare_local.sh"),
        )
        .arg("8788")
        .arg(wrangler_config)
        .current_dir(&workspace_root)
        .env("WRANGLER_LOG_DIR", wrangler_log_dir_str)
        .env("XDG_CONFIG_HOME", wrangler_config_dir_str)
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()?;
    let _guard = ChildGuard { child };

    if let Err(err) = wait_for_health("http://127.0.0.1:8788/health").await {
        let stdout = std::fs::read_to_string(&stdout_path).unwrap_or_default();
        let stderr = std::fs::read_to_string(&stderr_path).unwrap_or_default();
        return Err(format!(
            "{err}\nwrangler dev stdout:\n{stdout}\nwrangler dev stderr:\n{stderr}"
        )
        .into());
    }

    let resp = reqwest::get(format!(
        "http://127.0.0.1:8788/listing?auth_token={auth_token}"
    ))
    .await?;
    assert!(resp.status().is_success(), "Unexpected status: {}", resp.status());
    let body = resp.text().await?;
    assert!(body.contains("<table>"), "Listing HTML missing table");
    assert!(body.contains("401703504"), "Listing missing event 401703504");
    assert!(
        body.contains("Masters Tournament 2025"),
        "Listing missing Masters Tournament 2025"
    );
    assert!(body.contains("401703521"), "Listing missing event 401703521");
    assert!(
        body.contains("The Open 2025"),
        "Listing missing The Open 2025"
    );

    Ok(())
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
}

fn run_serverless_enabled() -> Result<bool, Box<dyn Error>> {
    let env_path = workspace_root().join(".env");
    let Ok(contents) = std::fs::read_to_string(env_path) else {
        return Ok(false);
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
            return Ok(true);
        }
    }
    Ok(false)
}

fn ensure_command(cmd: &str) -> Result<(), Box<dyn Error>> {
    let status = Command::new("which").arg(cmd).status()?;
    if !status.success() {
        return Err(format!("Required command not found: {cmd}").into());
    }
    Ok(())
}

fn run_script(
    script_path: &Path,
    envs: &[(&str, &str)],
    cwd: &Path,
) -> Result<(), Box<dyn Error>> {
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

struct ChildGuard {
    child: Child,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
