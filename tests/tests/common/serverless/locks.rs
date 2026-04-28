use super::event_id_i32;
use super::types::TestLockStatus;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

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

pub fn test_lock_token(test_name: &str) -> String {
    format!(
        "{}-{}-{}",
        test_name,
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or(0)
    )
}

pub async fn admin_test_lock(
    miniflare_url: &str,
    admin_token: &str,
    event_id: i64,
    token: &str,
    mode: &str,
    force: bool,
) -> Result<TestLockStatus, Box<dyn Error>> {
    let payload = AdminTestLockRequest {
        event_id: event_id_i32(event_id)?,
        token: token.to_string(),
        ttl_secs: 30,
        mode: mode.to_string(),
        force: force.then_some(true),
    };
    let resp = Client::new()
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
    let payload = AdminTestUnlockRequest {
        event_id: event_id_i32(event_id)?,
        token: token.to_string(),
    };
    let resp = Client::new()
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
