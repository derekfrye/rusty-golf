#![cfg(target_arch = "wasm32")]

use serde::{Deserialize, Serialize};
use worker::{Env, Request, Response, Result, RouteContext};

use crate::storage::{AdminSeedRequest, TestLockMode};
use crate::utils::{parse_query_params, storage_from_env};

fn admin_enabled(env: &Env) -> bool {
    env.var("ADMIN_ENABLED")
        .ok()
        .map(|value| value.to_string() == "1")
        .unwrap_or(false)
}

fn admin_token(env: &Env) -> Result<String> {
    let value = env
        .var("ADMIN_TOKEN")
        .map_err(|e| worker::Error::RustError(format!("Missing ADMIN_TOKEN env var: {e}")))?;
    let value = value.to_string();
    if value.trim().is_empty() {
        Err(worker::Error::RustError("ADMIN_TOKEN is empty".to_string()))
    } else {
        Ok(value)
    }
}

fn admin_request_token(req: &Request) -> Result<Option<String>> {
    if let Ok(Some(token)) = req.headers().get("x-admin-token") {
        if !token.trim().is_empty() {
            return Ok(Some(token));
        }
    }
    let query = parse_query_params(req)?;
    Ok(query.get("admin_token").cloned())
}

fn admin_auth_response(req: &Request, env: &Env) -> Result<Option<Response>> {
    if !admin_enabled(env) {
        return Ok(Some(Response::error("not found", 404)?));
    }
    let expected = admin_token(env)?;
    let Some(provided) = admin_request_token(req)? else {
        return Ok(Some(Response::error("unauthorized", 401)?));
    };
    if provided != expected {
        return Ok(Some(Response::error("unauthorized", 401)?));
    }
    Ok(None)
}

#[derive(Deserialize)]
struct AdminCleanupRequest {
    event_id: i32,
    #[serde(default)]
    include_auth_tokens: bool,
}

#[derive(Deserialize)]
struct AdminEndDateRequest {
    event_id: i32,
    end_date: Option<String>,
}

#[derive(Deserialize)]
struct AdminTestLockRequest {
    event_id: i32,
    token: String,
    ttl_secs: Option<i64>,
    mode: Option<String>,
    #[serde(default)]
    force: bool,
}

#[derive(Serialize)]
struct AdminTestLockResponse {
    acquired: bool,
    is_first: bool,
}

#[derive(Deserialize)]
struct AdminTestUnlockRequest {
    event_id: AdminEventSelector,
    token: String,
}

#[derive(Serialize)]
struct AdminTestUnlockResponse {
    is_last: bool,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum AdminEventSelector {
    All(String),
    Id(i32),
}

pub async fn admin_seed_handler(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Some(resp) = admin_auth_response(&req, &ctx.env)? {
        return Ok(resp);
    }
    let payload: AdminSeedRequest = req
        .json()
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let storage = storage_from_env(&ctx.env)?;
    storage
        .admin_seed_event(payload)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    Response::ok("seeded")
}

pub async fn admin_cleanup_handler(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Some(resp) = admin_auth_response(&req, &ctx.env)? {
        return Ok(resp);
    }
    let payload: AdminCleanupRequest = req
        .json()
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let storage = storage_from_env(&ctx.env)?;
    storage
        .admin_cleanup_event(payload.event_id, payload.include_auth_tokens)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    Response::ok("cleaned")
}

pub async fn admin_end_date_handler(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Some(resp) = admin_auth_response(&req, &ctx.env)? {
        return Ok(resp);
    }
    let payload: AdminEndDateRequest = req
        .json()
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let storage = storage_from_env(&ctx.env)?;
    storage
        .admin_update_event_end_date(payload.event_id, payload.end_date)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    Response::ok("updated")
}

pub async fn admin_test_lock_handler(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Some(resp) = admin_auth_response(&req, &ctx.env)? {
        return Ok(resp);
    }
    let payload: AdminTestLockRequest = req
        .json()
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let storage = storage_from_env(&ctx.env)?;
    let ttl_secs = payload.ttl_secs.unwrap_or(30);
    let mode = match payload.mode.as_deref() {
        Some("exclusive") => TestLockMode::Exclusive,
        _ => TestLockMode::Shared,
    };
    let (acquired, is_first) = storage
        .admin_test_lock(
            payload.event_id,
            &payload.token,
            ttl_secs,
            mode,
            payload.force,
        )
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    Response::from_json(&AdminTestLockResponse { acquired, is_first })
}

pub async fn admin_test_unlock_handler(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    if let Some(resp) = admin_auth_response(&req, &ctx.env)? {
        return Ok(resp);
    }
    let payload: AdminTestUnlockRequest = req
        .json()
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let storage = storage_from_env(&ctx.env)?;
    let is_last = match payload.event_id {
        AdminEventSelector::Id(event_id) => storage
            .admin_test_unlock(event_id, &payload.token)
            .await
            .map_err(|e| worker::Error::RustError(e.to_string()))?,
        AdminEventSelector::All(value) => {
            if value != "all" {
                return Response::error("event_id must be an integer or \"all\"", 400);
            }
            storage
                .admin_test_unlock_all()
                .await
                .map_err(|e| worker::Error::RustError(e.to_string()))?;
            true
        }
    };
    Response::from_json(&AdminTestUnlockResponse { is_last })
}
