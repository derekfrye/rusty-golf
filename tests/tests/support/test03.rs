use crate::BrysonExpectations;
use crate::common::serverless::admin::{admin_cleanup_events, admin_seed_event};
use crate::common::serverless::fixtures::{load_espn_cache, load_eup_event, load_score_struct};
use crate::common::serverless::locks::{admin_test_lock_retry, admin_test_unlock, test_lock_token};
use crate::common::serverless::types::{AdminSeedRequest, WranglerPaths};
use crate::common::serverless::{event_id_i32, is_local_miniflare, shared_wrangler_dirs};
use crate::test03_build::build_local;
use rusty_golf_actix::model::ScoreData;
use rusty_golf_actix::storage::SqlStorage;
use rusty_golf_actix::view::score::{
    render_scores_template_pure, scores_and_last_refresh_to_line_score_tables,
};
use rusty_golf_core::storage::Storage;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub(super) async fn run_miniflare_checks(
    storage: &SqlStorage,
    expectations: &BrysonExpectations,
) -> Result<(), Box<dyn Error>> {
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

    let test_result =
        run_seeded_checks(storage, expectations, &workspace_root, &miniflare_url).await;

    let is_last = admin_test_unlock(&miniflare_url, &admin_token, event_id, &lock_token).await?;
    if is_last
        && let Err(err) =
            admin_cleanup_events(&miniflare_url, &admin_token, &[event_id], false).await
    {
        eprintln!("admin cleanup failed after test03: {err}");
    }

    test_result
}

async fn run_seeded_checks(
    storage: &SqlStorage,
    expectations: &BrysonExpectations,
    workspace_root: &Path,
    miniflare_url: &str,
) -> Result<(), Box<dyn Error>> {
    let event_id = 401_580_351_i64;
    let payload = build_admin_seed_request(workspace_root, event_id)?;
    admin_seed_event(miniflare_url, &miniflare_admin_token()?, &payload).await?;
    let miniflare_scores = fetch_miniflare_scores(event_id, miniflare_url).await?;
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

fn build_admin_seed_request(
    workspace_root: &Path,
    event_id: i64,
) -> Result<AdminSeedRequest, Box<dyn Error>> {
    Ok(AdminSeedRequest {
        event_id: event_id_i32(event_id)?,
        refresh_from_espn: 1,
        event: load_eup_event(workspace_root, event_id)?,
        score_struct: load_score_struct(workspace_root)?,
        espn_cache: load_espn_cache(workspace_root)?,
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
        expectations.total_score,
        bryson.detailed_statistics.total_score
    );
    assert_eq!(expectations.reference_eup_id, bryson.eup_id);
    let line_score = bryson
        .detailed_statistics
        .line_scores
        .iter()
        .find(|s| s.hole == 13 && s.round == 2)
        .expect("Serverless line score missing")
        .score;
    assert_eq!(expectations.line_score, line_score);
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
}

fn ensure_command(cmd: &str) -> Result<(), Box<dyn Error>> {
    let status = Command::new("which").arg(cmd).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Required command not found: {cmd}").into())
    }
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
    std::env::var("MINIFLARE_ADMIN_TOKEN")
        .map_err(|_| "MINIFLARE_ADMIN_TOKEN not set in ../.env or environment".into())
}
