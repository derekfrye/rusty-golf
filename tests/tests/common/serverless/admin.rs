use super::event_id_i32;
use super::types::AdminSeedRequest;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

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

#[derive(Debug, Deserialize)]
struct AdminListingResponse {
    scores_exists: Option<bool>,
    espn_cache_exists: Option<bool>,
}

#[derive(Debug, Serialize)]
struct AdminUpdateDatesRequest {
    event_id: i32,
    start_date: Option<String>,
    end_date: Option<String>,
    completed: Option<bool>,
}

pub async fn admin_seed_event(
    miniflare_url: &str,
    admin_token: &str,
    payload: &AdminSeedRequest,
) -> Result<(), Box<dyn Error>> {
    let resp = Client::new()
        .post(format!("{miniflare_url}/admin/seed"))
        .header("x-admin-token", admin_token)
        .json(payload)
        .send()
        .await?;
    ensure_success(resp, "admin seed").await
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
            event_id: event_id_i32(*event_id)?,
            include_auth_tokens,
        };
        let resp = client
            .post(format!("{miniflare_url}/admin/cleanup"))
            .header("x-admin-token", admin_token)
            .json(&payload)
            .send()
            .await?;
        ensure_success(resp, "admin cleanup").await?;
    }
    Ok(())
}

pub async fn admin_cleanup_scores(
    miniflare_url: &str,
    admin_token: &str,
    event_id: i64,
) -> Result<(), Box<dyn Error>> {
    let payload = AdminCleanupScoresRequest {
        event_id: event_id_i32(event_id)?,
    };
    let resp = Client::new()
        .post(format!("{miniflare_url}/admin/cleanup_scores"))
        .header("x-admin-token", admin_token)
        .json(&payload)
        .send()
        .await?;
    ensure_success(resp, "admin cleanup scores").await
}

pub async fn admin_set_espn_failure(
    miniflare_url: &str,
    admin_token: &str,
    event_id: i64,
    enabled: bool,
) -> Result<(), Box<dyn Error>> {
    let payload = AdminEspnFailRequest {
        event_id: event_id_i32(event_id)?,
        enabled,
    };
    let resp = Client::new()
        .post(format!("{miniflare_url}/admin/espn_fail"))
        .header("x-admin-token", admin_token)
        .json(&payload)
        .send()
        .await?;
    ensure_success(resp, "admin espn fail").await
}

pub async fn admin_scores_exists(
    miniflare_url: &str,
    admin_token: &str,
    event_id: i64,
) -> Result<(bool, bool), Box<dyn Error>> {
    let event_id_i32 = event_id_i32(event_id)?;
    let url = format!("{miniflare_url}/listing?event_id={event_id_i32}");
    let resp = Client::new()
        .get(url)
        .header("x-admin-token", admin_token)
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(format!("admin listing failed: {}", resp.status()).into());
    }
    let body: AdminListingResponse = resp.json().await?;
    Ok((
        body.scores_exists.unwrap_or(false),
        body.espn_cache_exists.unwrap_or(false),
    ))
}

pub async fn admin_update_dates(
    miniflare_url: &str,
    admin_token: &str,
    event_id: i64,
    start_date: Option<String>,
    end_date: Option<String>,
    completed: Option<bool>,
) -> Result<(), Box<dyn Error>> {
    let payload = AdminUpdateDatesRequest {
        event_id: event_id_i32(event_id)?,
        start_date,
        end_date,
        completed,
    };
    let resp = Client::new()
        .post(format!("{miniflare_url}/admin/event_update_dates"))
        .header("x-admin-token", admin_token)
        .json(&payload)
        .send()
        .await?;
    ensure_success(resp, "admin date update").await
}

async fn ensure_success(resp: reqwest::Response, context: &str) -> Result<(), Box<dyn Error>> {
    if resp.status().is_success() {
        return Ok(());
    }
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    Err(format!("{context} failed: {status}\n{body}").into())
}
