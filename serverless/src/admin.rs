#![cfg(target_arch = "wasm32")]

use worker::{Request, Response, Result, RouteContext};

use crate::admin_auth::admin_auth_response;
use crate::admin_types::{
    AdminCacheFlushRequest, AdminCleanupRequest, AdminCleanupScoresRequest, AdminEspnFailRequest,
    AdminEventSelector, AdminTestLockRequest, AdminTestLockResponse, AdminTestUnlockRequest,
    AdminTestUnlockResponse, AdminUpdateDatesRequest,
};
use crate::storage::{AdminSeedRequest, TestLockMode};
use crate::utils::storage_from_env;

mod cache_status;
pub use cache_status::admin_cache_status_handler;

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

pub async fn admin_cleanup_scores_handler(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    if let Some(resp) = admin_auth_response(&req, &ctx.env)? {
        return Ok(resp);
    }
    let payload: AdminCleanupScoresRequest = req
        .json()
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let storage = storage_from_env(&ctx.env)?;
    storage
        .admin_cleanup_scores(payload.event_id)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    Response::ok("cleaned scores")
}

pub async fn admin_cache_flush_handler(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    if let Some(resp) = admin_auth_response(&req, &ctx.env)? {
        return Ok(resp);
    }
    let payload: AdminCacheFlushRequest = req
        .json()
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let storage = storage_from_env(&ctx.env)?;
    storage
        .admin_flush_scores_cache(payload.event_id)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    Response::ok("cache flushed")
}

pub async fn admin_espn_fail_handler(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Some(resp) = admin_auth_response(&req, &ctx.env)? {
        return Ok(resp);
    }
    let payload: AdminEspnFailRequest = req
        .json()
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let storage = storage_from_env(&ctx.env)?;
    storage
        .admin_set_espn_failure(payload.event_id, payload.enabled)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    Response::ok("updated")
}

pub async fn admin_update_dates_handler(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    if let Some(resp) = admin_auth_response(&req, &ctx.env)? {
        return Ok(resp);
    }
    let payload: AdminUpdateDatesRequest = req
        .json()
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let storage = storage_from_env(&ctx.env)?;
    storage
        .admin_update_event_dates(
            payload.event_id,
            payload.start_date,
            payload.end_date,
            payload.completed,
        )
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
