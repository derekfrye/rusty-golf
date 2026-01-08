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

#[derive(Debug, Deserialize)]
struct EspnFixture {
    score_struct: Vec<Scores>,
}

pub fn load_eup_event(
    workspace_root: &Path,
    event_id: i64,
) -> Result<EupEventInput, Box<dyn Error>> {
    let path = workspace_root.join("rusty-golf-tests/tests/test05_dbprefill.json");
    let contents = fs::read_to_string(path)?;
    let events: Vec<EupEventInput> = serde_json::from_str(&contents)?;
    events
        .into_iter()
        .find(|event| event.event == event_id)
        .ok_or_else(|| format!("Missing event {event_id} in test05_dbprefill.json").into())
}

pub fn load_score_struct(workspace_root: &Path) -> Result<Vec<Scores>, Box<dyn Error>> {
    let path = workspace_root.join("rusty-golf-tests/tests/test03_espn_json_responses.json");
    let contents = fs::read_to_string(path)?;
    let fixture: EspnFixture = serde_json::from_str(&contents)?;
    Ok(fixture.score_struct)
}

pub fn load_espn_cache(workspace_root: &Path) -> Result<serde_json::Value, Box<dyn Error>> {
    let path = workspace_root.join("rusty-golf-tests/tests/test03_espn_json_responses.json");
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
        return Err(format!("admin seed failed: {}\n{}", status, body).into());
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
        let payload = AdminCleanupRequest {
            event_id: *event_id as i32,
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
            return Err(format!("admin cleanup failed: {}\n{}", status, body).into());
        }
    }
    Ok(())
}
