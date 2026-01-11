use chrono::Utc;
use reqwest::Client;
use rusty_golf_core::model::Scores;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct WranglerPaths {
    pub config: PathBuf,
    pub log_dir: PathBuf,
    pub config_dir: PathBuf,
}

pub struct AdminCleanupGuard {
    miniflare_url: String,
    admin_token: String,
    event_ids: Vec<i64>,
    include_auth_tokens: bool,
}

pub fn shared_wrangler_dirs() -> Option<(PathBuf, PathBuf)> {
    let miniflare_root = PathBuf::from("/miniflare_work/.wrangler");
    let miniflare_logs = miniflare_root.join("logs");
    let miniflare_config = miniflare_root.join("config");
    if miniflare_logs.is_dir() && miniflare_config.is_dir() {
        return Some((miniflare_logs, miniflare_config));
    }
    let home = env::var("HOME").ok()?;
    let base = PathBuf::from(home).join(".local/share/rusty-golf-miniflare");
    Some((base.join("logs"), base.join("config")))
}

impl AdminCleanupGuard {
    pub fn new(
        miniflare_url: String,
        admin_token: String,
        event_ids: Vec<i64>,
        include_auth_tokens: bool,
    ) -> Self {
        Self {
            miniflare_url,
            admin_token,
            event_ids,
            include_auth_tokens,
        }
    }
}

impl Drop for AdminCleanupGuard {
    fn drop(&mut self) {
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let miniflare_url = self.miniflare_url.clone();
        let admin_token = self.admin_token.clone();
        let event_ids = self.event_ids.clone();
        let include_auth_tokens = self.include_auth_tokens;
        handle.spawn(async move {
            let _ = admin_cleanup_events(
                &miniflare_url,
                &admin_token,
                &event_ids,
                include_auth_tokens,
            )
            .await;
        });
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EupEventInput {
    pub event: i64,
    pub name: String,
    pub score_view_step_factor: serde_json::Value,
    pub end_date: Option<String>,
    pub data_to_fill_if_event_and_year_missing: Vec<EupDataFillInput>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EupDataFillInput {
    pub golfers: Vec<EupGolferInput>,
    pub event_user_player: Vec<EupEventUserPlayerInput>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EupGolferInput {
    pub espn_id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EupEventUserPlayerInput {
    pub bettor: String,
    pub golfer_espn_id: i64,
    pub score_view_step_factor: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct AdminSeedRequest {
    pub event_id: i32,
    pub refresh_from_espn: i64,
    pub event: EupEventInput,
    pub score_struct: Vec<Scores>,
    pub espn_cache: serde_json::Value,
    pub auth_tokens: Option<Vec<String>>,
    pub last_refresh: Option<String>,
}

#[derive(Debug, Serialize)]
struct AdminCleanupRequest {
    event_id: i32,
    include_auth_tokens: bool,
}

#[derive(Debug, Serialize)]
struct AdminCleanupScoresRequest {
    event_id: i32,
}

#[derive(Debug, Serialize)]
struct AdminEspnFailRequest {
    event_id: i32,
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct AdminTestLockRequest {
    event_id: i32,
    token: String,
    ttl_secs: i64,
    mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    force: Option<bool>,
}

#[derive(Debug, Serialize)]
struct AdminTestUnlockRequest {
    event_id: i32,
    token: String,
}

#[derive(Debug, Deserialize)]
struct AdminTestLockResponse {
    acquired: bool,
    is_first: bool,
}

#[derive(Debug, Deserialize)]
struct AdminTestUnlockResponse {
    is_last: bool,
}

#[derive(Debug, Serialize)]
struct AdminEndDateRequest {
    event_id: i32,
    end_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EspnFixture {
    score_struct: Vec<Scores>,
}

pub fn load_eup_event(
    workspace_root: &Path,
    event_id: i64,
) -> Result<EupEventInput, Box<dyn Error>> {
    let path = workspace_root.join("tests/tests/test05_dbprefill.json");
    let contents = fs::read_to_string(path)?;
    let events: Vec<EupEventInput> = serde_json::from_str(&contents)?;
    events
        .into_iter()
        .find(|event| event.event == event_id)
        .ok_or_else(|| format!("Missing event {event_id} in test05_dbprefill.json").into())
}

pub fn load_score_struct(workspace_root: &Path) -> Result<Vec<Scores>, Box<dyn Error>> {
    let path = workspace_root.join("tests/tests/test03_espn_json_responses.json");
    let contents = fs::read_to_string(path)?;
    let fixture: EspnFixture = serde_json::from_str(&contents)?;
    Ok(fixture.score_struct)
}

pub fn load_espn_cache(workspace_root: &Path) -> Result<serde_json::Value, Box<dyn Error>> {
    let path = workspace_root.join("tests/tests/test03_espn_json_responses.json");
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}

pub async fn admin_seed_event(
    miniflare_url: &str,
    admin_token: &str,
    payload: &AdminSeedRequest,
) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let resp = client
        .post(format!("{miniflare_url}/admin/seed"))
        .header("x-admin-token", admin_token)
        .json(payload)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("admin seed failed: {status}\n{body}").into());
    }
    Ok(())
}

pub async fn admin_cleanup_events(
    miniflare_url: &str,
    admin_token: &str,
    event_ids: &[i64],
    include_auth_tokens: bool,
) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    for event_id in event_ids {
        let event_id_i32 = event_id_i32(*event_id)?;
        let payload = AdminCleanupRequest {
            event_id: event_id_i32,
            include_auth_tokens,
        };
        let resp = client
            .post(format!("{miniflare_url}/admin/cleanup"))
            .header("x-admin-token", admin_token)
            .json(&payload)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("admin cleanup failed: {status}\n{body}").into());
        }
    }
    Ok(())
}

pub async fn admin_cleanup_scores(
    miniflare_url: &str,
    admin_token: &str,
    event_id: i64,
) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let event_id_i32 = event_id_i32(event_id)?;
    let payload = AdminCleanupScoresRequest { event_id: event_id_i32 };
    let resp = client
        .post(format!("{miniflare_url}/admin/cleanup_scores"))
        .header("x-admin-token", admin_token)
        .json(&payload)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("admin cleanup scores failed: {status}\n{body}").into());
    }
    Ok(())
}

pub async fn admin_set_espn_failure(
    miniflare_url: &str,
    admin_token: &str,
    event_id: i64,
    enabled: bool,
) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let event_id_i32 = event_id_i32(event_id)?;
    let payload = AdminEspnFailRequest {
        event_id: event_id_i32,
        enabled,
    };
    let resp = client
        .post(format!("{miniflare_url}/admin/espn_fail"))
        .header("x-admin-token", admin_token)
        .json(&payload)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("admin espn fail failed: {status}\n{body}").into());
    }
    Ok(())
}

pub fn test_lock_token(test_name: &str) -> String {
    format!(
        "{}-{}-{}",
        test_name,
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or(0)
    )
}

pub fn is_local_miniflare(url: &str) -> bool {
    let host = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

pub struct TestLockStatus {
    pub acquired: bool,
    pub is_first: bool,
}

pub async fn admin_test_lock(
    miniflare_url: &str,
    admin_token: &str,
    event_id: i64,
    token: &str,
    mode: &str,
    force: bool,
) -> Result<TestLockStatus, Box<dyn Error>> {
    let client = Client::new();
    let event_id_i32 = event_id_i32(event_id)?;
    let payload = AdminTestLockRequest {
        event_id: event_id_i32,
        token: token.to_string(),
        ttl_secs: 30,
        mode: mode.to_string(),
        force: force.then_some(true),
    };
    let resp = client
        .post(format!("{miniflare_url}/admin/test_lock"))
        .header("x-admin-token", admin_token)
        .json(&payload)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("admin test_lock failed: {status}\n{body}").into());
    }
    let body: AdminTestLockResponse = resp.json().await?;
    Ok(TestLockStatus {
        acquired: body.acquired,
        is_first: body.is_first,
    })
}

pub async fn admin_test_unlock(
    miniflare_url: &str,
    admin_token: &str,
    event_id: i64,
    token: &str,
) -> Result<bool, Box<dyn Error>> {
    let client = Client::new();
    let event_id_i32 = event_id_i32(event_id)?;
    let payload = AdminTestUnlockRequest {
        event_id: event_id_i32,
        token: token.to_string(),
    };
    let resp = client
        .post(format!("{miniflare_url}/admin/test_unlock"))
        .header("x-admin-token", admin_token)
        .json(&payload)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("admin test_unlock failed: {status}\n{body}").into());
    }
    let body: AdminTestUnlockResponse = resp.json().await?;
    Ok(body.is_last)
}

pub async fn admin_test_lock_retry(
    miniflare_url: &str,
    admin_token: &str,
    event_id: i64,
    token: &str,
    mode: &str,
) -> Result<TestLockStatus, Box<dyn Error>> {
    let mut attempts = 0;
    let max_attempts = 40;
    loop {
        let status =
            admin_test_lock(miniflare_url, admin_token, event_id, token, mode, false).await?;
        if status.acquired {
            return Ok(status);
        }
        attempts += 1;
        if attempts >= max_attempts {
            return Err(format!("Timed out waiting for {mode} lock on event {event_id}").into());
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

pub async fn admin_update_end_date(
    miniflare_url: &str,
    admin_token: &str,
    event_id: i64,
    end_date: Option<String>,
) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let event_id_i32 = event_id_i32(event_id)?;
    let payload = AdminEndDateRequest {
        event_id: event_id_i32,
        end_date,
    };
    let resp = client
        .post(format!("{miniflare_url}/admin/event_end_date"))
        .header("x-admin-token", admin_token)
        .json(&payload)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("admin end_date update failed: {status}\n{body}").into());
    }
    Ok(())
}

pub fn event_id_i32(event_id: i64) -> Result<i32, Box<dyn Error>> {
    i32::try_from(event_id).map_err(|_| format!("event_id out of range: {event_id}").into())
}
