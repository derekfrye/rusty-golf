use rusty_golf_core::model::Scores;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
            let _ = super::admin::admin_cleanup_events(
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
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    #[serde(default)]
    pub completed: bool,
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

pub struct TestLockStatus {
    pub acquired: bool,
    pub is_first: bool,
}
