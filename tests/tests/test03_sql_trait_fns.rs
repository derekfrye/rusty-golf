mod common;

use common::serverless::{
    AdminSeedRequest, WranglerPaths, admin_cleanup_events, admin_seed_event, admin_test_lock_retry,
    admin_test_unlock, event_id_i32, is_local_miniflare, load_espn_cache, load_eup_event,
    load_score_struct, shared_wrangler_dirs, test_lock_token,
};
use rusty_golf_actix::controller::score::get_data_for_scores_page;
use rusty_golf_actix::model::ScoreData;
use rusty_golf_actix::storage::SqlStorage;
use rusty_golf_actix::view::score::{
    render_scores_template_pure, scores_and_last_refresh_to_line_score_tables,
};
use serde_json::Value;
use sql_middleware::middleware::{ConfigAndPool as ConfigAndPool2, QueryAndParams, SqliteOptions};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[tokio::test]
async fn test3_sqlx_trait_get_scores() -> Result<(), Box<dyn std::error::Error>> {
    init_env();

    let storage = setup_sqlite_storage().await?;
    println!("Running SQL-backed assertions");
    let score_data = get_data_for_scores_page(401_580_351, 2024, false, &storage, 0)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let reference_result = reference_json()?;
    let expectations = assert_bryson_scores(&score_data, &reference_result);

    if run_serverless_enabled() {
        run_miniflare_checks(&storage, &expectations).await?;
    } else {
        println!("Skipping serverless checks: RUN_SERVERLESS=1 not set in .env");
    }

    Ok(())
}

struct BrysonExpectations {
    total_score: i32,
    line_score: i32,
    reference_eup_id: i64,
}

fn init_env() {
    let _ = dotenvy::dotenv();
    if std::env::var("MINIFLARE_URL").is_err() || std::env::var("MINIFLARE_ADMIN_TOKEN").is_err() {
        let _ = dotenvy::from_filename("../.env");
    }
}

fn run_serverless_enabled() -> bool {
    init_env();
    std::env::var("RUN_SERVERLESS")
        .map(|value| value.trim() == "1")
        .unwrap_or(false)
}

async fn setup_sqlite_storage() -> Result<SqlStorage, Box<dyn std::error::Error>> {
    let sqlite_options = SqliteOptions::new("file::memory:?cache=shared".to_string());
    let config_and_pool = ConfigAndPool2::new_sqlite(sqlite_options).await.unwrap();

    let ddl = [
        include_str!("../../actix/src/sql/schema/sqlite/00_event.sql"),
        // include_str!("../../actix/src/sql/schema/sqlite/01_golfstatistic.sql"),
        include_str!("../../actix/src/sql/schema/sqlite/02_golfer.sql"),
        include_str!("../../actix/src/sql/schema/sqlite/03_bettor.sql"),
        include_str!("../../actix/src/sql/schema/sqlite/04_event_user_player.sql"),
        include_str!("../../actix/src/sql/schema/sqlite/05_eup_statistic.sql"),
    ];

    let mut conn = config_and_pool.get_connection().await?;
    let query_and_params = QueryAndParams {
        query: ddl.join("\n"),
        params: vec![],
    };
    conn.execute_batch(&query_and_params.query).await?;

    let setup_queries = include_str!("test01.sql");
    let query_and_params = QueryAndParams {
        query: setup_queries.to_string(),
        params: vec![],
    };
    conn.execute_batch(&query_and_params.query).await?;

    Ok(SqlStorage::new(config_and_pool.clone()))
}

fn reference_json() -> Result<Value, Box<dyn std::error::Error>> {
    let reference_result_str = include_str!("test03_espn_json_responses.json");
    Ok(serde_json::from_str(reference_result_str)?)
}

fn assert_bryson_scores(score_data: &ScoreData, reference_result: &Value) -> BrysonExpectations {
    let bryson_espn_entry = score_data
        .score_struct
        .iter()
        .find(|s| s.golfer_name == "Bryson DeChambeau")
        .expect("Score data missing Bryson DeChambeau");
    let bryson_reference_entry = reference_result
        .get("score_struct")
        .and_then(Value::as_array)
        .and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry.get("golfer_name") == Some(&Value::from("Bryson DeChambeau")))
        })
        .expect("Reference JSON missing Bryson DeChambeau");

    let reference_total = bryson_reference_entry
        .get("detailed_statistics")
        .and_then(|stats| stats.get("total_score"))
        .and_then(Value::as_i64)
        .expect("Reference entry missing total_score");

    assert_eq!(
        i64::from(bryson_espn_entry.detailed_statistics.total_score),
        reference_total
    );

    let reference_eup_id = bryson_reference_entry
        .get("eup_id")
        .and_then(Value::as_i64)
        .expect("Reference entry missing eup_id");
    assert_eq!(bryson_espn_entry.eup_id, reference_eup_id);

    let reference_line_score = bryson_reference_entry
        .get("detailed_statistics")
        .and_then(|stats| stats.get("line_scores"))
        .and_then(Value::as_array)
        .and_then(|scores| {
            scores.iter().find(|s| {
                s.get("hole").and_then(Value::as_i64) == Some(13)
                    && s.get("round").and_then(Value::as_i64) == Some(2)
            })
        })
        .and_then(|entry| entry.get("score"))
        .and_then(Value::as_i64)
        .expect("Reference entry missing line score");
    let reference_line_score = i32::try_from(reference_line_score).expect("score fits in i32");

    let line_score = bryson_espn_entry
        .detailed_statistics
        .line_scores
        .iter()
        .find(|s| s.hole == 13 && s.round == 2)
        .expect("Score data missing line score")
        .score;
    assert_eq!(reference_line_score, line_score);
    assert_eq!(reference_line_score, 3); // line 6824 in test03_espn_json_responses.json

    BrysonExpectations {
        total_score: bryson_espn_entry.detailed_statistics.total_score,
        line_score,
        reference_eup_id,
    }
}

async fn run_miniflare_checks(
    storage: &SqlStorage,
    expectations: &BrysonExpectations,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running serverless-backed assertions");
    ensure_command("worker-build")?;
    ensure_command("wrangler")?;

    let workspace_root = workspace_root();
    let miniflare_url = miniflare_base_url()?;
    let admin_token = miniflare_admin_token()?;
    let wrangler_paths = wrangler_paths(&workspace_root);
    let event_id = 401_580_351_i64;
    let lock_token = test_lock_token("test03");

    if is_local_miniflare(&miniflare_url) {
        build_local(&workspace_root, &wrangler_paths)?;
    } else {
        println!("Skipping build_local; MINIFLARE_URL is non-localhost.");
    }
    wait_for_health(&format!("{miniflare_url}/health")).await?;
    println!("miniflare health check passed");

    let _lock = admin_test_lock_retry(
        &miniflare_url,
        &admin_token,
        event_id,
        &lock_token,
        "exclusive",
    )
    .await?;
    admin_cleanup_events(&miniflare_url, &admin_token, &[event_id], false).await?;

    let test_result = async {
        let payload = build_admin_seed_request(&workspace_root, event_id)?;
        admin_seed_event(&miniflare_url, &admin_token, &payload).await?;
        let miniflare_scores = fetch_miniflare_scores(event_id, &miniflare_url).await?;
        assert_miniflare_scores(&miniflare_scores, expectations);

        let from_db_scores = storage
            .get_scores(401_580_351, rusty_golf_actix::model::RefreshSource::Db)
            .await?;
        let bettor_struct = scores_and_last_refresh_to_line_score_tables(&from_db_scores);
        let event_details = storage.get_event_details(401_580_351).await?;
        let player_step_factors = storage.get_player_step_factors(401_580_351).await?;

        let markup = render_scores_template_pure(
            &miniflare_scores,
            false,
            &bettor_struct,
            event_details.score_view_step_factor,
            &player_step_factors,
            401_580_351,
            2024,
            true,
        );

        assert!(
            !markup.into_string().is_empty(),
            "Serverless-rendered markup should not be empty"
        );
        Ok(())
    }
    .await;

    let is_last = admin_test_unlock(&miniflare_url, &admin_token, event_id, &lock_token).await?;
    if is_last
        && let Err(err) =
            admin_cleanup_events(&miniflare_url, &admin_token, &[event_id], false).await
    {
        eprintln!("admin cleanup failed after test03: {err}");
    }

    test_result
}

fn build_admin_seed_request(
    workspace_root: &Path,
    event_id: i64,
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
        last_refresh: None,
    })
}

async fn fetch_miniflare_scores(
    event_id: i64,
    miniflare_url: &str,
) -> Result<ScoreData, Box<dyn Error>> {
    let resp = reqwest::get(format!(
        "{miniflare_url}/scores?event={event_id}&yr=2024&cache=1&json=1"
    ))
    .await?;
    if !resp.status().is_success() {
        return Err(format!("Unexpected status: {}", resp.status()).into());
    }
    Ok(resp.json::<ScoreData>().await?)
}

fn assert_miniflare_scores(scores: &ScoreData, expectations: &BrysonExpectations) {
    let bryson = scores
        .score_struct
        .iter()
        .find(|s| s.golfer_name == "Bryson DeChambeau")
        .expect("Serverless scores missing Bryson DeChambeau");
    assert_eq!(
        expectations.total_score, bryson.detailed_statistics.total_score,
        "Serverless total score mismatch for Bryson DeChambeau"
    );
    assert_eq!(
        expectations.reference_eup_id, bryson.eup_id,
        "Serverless eup_id mismatch for Bryson DeChambeau"
    );
    let line_score = bryson
        .detailed_statistics
        .line_scores
        .iter()
        .find(|s| s.hole == 13 && s.round == 2)
        .expect("Serverless line score missing")
        .score;
    assert_eq!(
        expectations.line_score, line_score,
        "Serverless line score mismatch"
    );
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
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
            workspace_root.join(".wrangler-logs-test03"),
            workspace_root.join(".wrangler-config-test03"),
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
