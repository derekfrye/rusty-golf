use serde_json::Value;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn test1_serverless_scores_endpoint() -> Result<(), Box<dyn Error>> {
    if !run_serverless_enabled()? {
        eprintln!("Skipping serverless test: RUN_SERVERLESS=1 not set in .env");
        return Ok(());
    }

    ensure_command("worker-build")?;
    ensure_command("wrangler")?;
    ensure_command("jq")?;

    let workspace_root = workspace_root();
    let wrangler_config = workspace_root.join("rusty-golf-serverless/wrangler.toml");

    let wrangler_log_dir = workspace_root.join(".wrangler-logs");
    let wrangler_log_dir_str = wrangler_log_dir.to_str().unwrap_or_default();
    let wrangler_config_dir = workspace_root.join(".wrangler-config");
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
        "--local --preview false --config {}",
        wrangler_config.display()
    );
    let wrangler_r2_flags = format!("--local --config {}", wrangler_config.display());
    run_script(
        &workspace_root.join("rusty-golf-serverless/scripts/seed_test1_local.sh"),
        &[
            ("WRANGLER_KV_FLAGS", wrangler_kv_flags.as_str()),
            ("WRANGLER_R2_FLAGS", wrangler_r2_flags.as_str()),
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
        .arg("8787")
        .arg(wrangler_config)
        .current_dir(&workspace_root)
        .env("WRANGLER_LOG_DIR", wrangler_log_dir_str)
        .env("XDG_CONFIG_HOME", wrangler_config_dir_str)
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()?;
    let _guard = ChildGuard { child };

    if let Err(err) = wait_for_health("http://127.0.0.1:8787/health").await {
        let stdout = std::fs::read_to_string(&stdout_path).unwrap_or_default();
        let stderr = std::fs::read_to_string(&stderr_path).unwrap_or_default();
        return Err(format!(
            "{err}\nwrangler dev stdout:\n{stdout}\nwrangler dev stderr:\n{stderr}"
        )
        .into());
    }

    let resp = reqwest::get("http://127.0.0.1:8787/scores?event=401580351&yr=2024&cache=1&json=1")
        .await?;
    assert!(resp.status().is_success(), "Unexpected status: {}", resp.status());
    let body: Value = resp.json().await?;
    assert!(body.is_object(), "Response is not a JSON object; got {body:?}");

    let bettor_struct = body
        .get("bettor_struct")
        .and_then(|v| v.as_array())
        .expect("Response JSON does not contain 'bettor_struct' array");
    assert_eq!(bettor_struct.len(), 5, "Unexpected number of bettors returned");

    let reference_result: Value =
        serde_json::from_str(include_str!("test1_expected_output.json"))?;
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

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
}

fn run_serverless_enabled() -> Result<bool, Box<dyn Error>> {
    let env_path = workspace_root().join(".env");
    let contents = match std::fs::read_to_string(env_path) {
        Ok(val) => val,
        Err(_) => return Ok(false),
    };
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            if key.trim() == "RUN_SERVERLESS" && value.trim() == "1" {
                return Ok(true);
            }
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
