use rusty_golf_setup::{SeedOptions, seed_kv_from_eup};
use serde_json::Value;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn test1_serverless_scores_endpoint() -> Result<(), Box<dyn Error>> {
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
    let event_id = 401_580_351_i64;

    build_local(&workspace_root, &wrangler_paths)?;
    seed_kv(&workspace_root, &wrangler_paths, &wrangler_flags, event_id)?;
    seed_r2(&workspace_root, &wrangler_paths, &wrangler_flags)?;

    let (guard, log_paths) = start_miniflare(&workspace_root, &wrangler_paths, "8787")?;
    let _guard = guard;

    if let Err(err) = wait_for_health("http://127.0.0.1:8787/health").await {
        let (stdout, stderr) = read_wrangler_logs(&log_paths);
        return Err(format!(
            "{err}\nwrangler dev stdout:\n{stdout}\nwrangler dev stderr:\n{stderr}"
        )
        .into());
    }
    println!("wrangler dev health check passed");

    assert_scores_response(event_id).await?;

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
        log_dir: workspace_root.join(".wrangler-logs"),
        config_dir: workspace_root.join(".wrangler-config"),
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

fn seed_kv(
    workspace_root: &Path,
    wrangler_paths: &WranglerPaths,
    wrangler_flags: &WranglerFlags,
    event_id: i64,
) -> Result<(), Box<dyn Error>> {
    seed_kv_from_eup(&SeedOptions {
        eup_json: workspace_root.join("rusty-golf-tests/tests/test5_dbprefill.json"),
        kv_env: "dev".to_string(),
        kv_binding: Some("djf_rusty_golf_kv".to_string()),
        auth_tokens: None,
        event_id: Some(event_id),
        refresh_from_espn: 1,
        wrangler_config: wrangler_paths.config.clone(),
        wrangler_env: "dev".to_string(),
        wrangler_kv_flags: wrangler_flags.kv_flag_list.clone(),
        wrangler_log_dir: Some(wrangler_paths.log_dir.clone()),
        wrangler_config_dir: Some(wrangler_paths.config_dir.clone()),
    })
    .map_err(|err| -> Box<dyn Error> { err.into() })?;
    Ok(())
}

fn seed_r2(
    workspace_root: &Path,
    wrangler_paths: &WranglerPaths,
    wrangler_flags: &WranglerFlags,
) -> Result<(), Box<dyn Error>> {
    let now = chrono::Local::now();
    println!(
        "Seeding test data via seed_test1_local.sh at {}",
        now.format("%H:%M:%S")
    );
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
    )?;
    let now = chrono::Local::now();
    println!("seed_test1_local completed at {}", now.format("%H:%M:%S"));
    Ok(())
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

    let now = chrono::Local::now();
    println!("Starting wrangler dev at {}", now.format("%H:%M:%S"));
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
    let now = chrono::Local::now();
    println!("wrangler dev started at {}", now.format("%H:%M:%S"));

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

async fn assert_scores_response(event_id: i64) -> Result<(), Box<dyn Error>> {
    let resp = reqwest::get(format!(
        "http://127.0.0.1:8787/scores?event={event_id}&yr=2024&cache=1&json=1"
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

    let reference_result: Value = serde_json::from_str(include_str!("test1_expected_output.json"))?;
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

struct ChildGuard {
    child: Child,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
