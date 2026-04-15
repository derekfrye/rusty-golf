#![cfg(target_arch = "wasm32")]

use serde::Serialize;
use std::collections::HashMap;
use std::rc::Rc;
use worker::{Env, Request, Response, Result, RouteContext};

use crate::instrument::RequestInstrumentation;
use crate::instrument::request_instrumentation;
use crate::storage::ServerlessStorage;
use crate::storage::storage_cache::{CacheStatus, in_memory_status, parse_kv_scores_entry};
use crate::utils::{parse_query_params, storage_from_env};
use rusty_golf_core::timed;
use rusty_golf_core::timing::TimingSink;

#[derive(Serialize)]
struct CacheStatusKeys {
    kv: String,
    r2_scores: String,
}

#[derive(Serialize)]
struct CacheStatusResponse {
    event_id: i32,
    year: i32,
    in_memory: CacheStatus,
    kv: CacheStatus,
    r2: CacheStatus,
    keys: CacheStatusKeys,
}

fn auth_details(event_id: i32, year: i32, status: i32) -> serde_json::Value {
    serde_json::json!({
        "event_id": event_id,
        "year": year,
        "admin": true,
        "status": status,
    })
}

async fn require_authorized(
    instrumentation: &RequestInstrumentation,
    req: &Request,
    env: &Env,
    storage: &ServerlessStorage,
    timing: Option<&dyn TimingSink>,
    query: &HashMap<String, String>,
    event_id: i32,
    year: i32,
) -> Result<Option<Response>> {
    if instrumentation.instrument_header_valid() {
        return Ok(None);
    }
    let auth_token = match query.get("auth_token") {
        Some(value) if !value.trim().is_empty() => value.trim(),
        _ => {
            let details = auth_details(event_id, year, 401);
            let response = crate::finalize_resp!(
                instrumentation,
                req,
                env,
                details,
                Response::error("auth_token is required", 401)
            )?;
            return Ok(Some(response));
        }
    };
    let auth_ok = timed!(
        timing,
        "storage.auth_token_valid_ms",
        storage
            .auth_token_valid(auth_token)
            .await
            .map_err(|e| worker::Error::RustError(e.to_string()))
    )?;
    if !auth_ok {
        let details = auth_details(event_id, year, 401);
        let response = crate::finalize_resp!(
            instrumentation,
            req,
            env,
            details,
            Response::error("auth_token is invalid", 401)
        )?;
        return Ok(Some(response));
    }
    Ok(None)
}

async fn build_cache_status_response(
    storage: &ServerlessStorage,
    timing: Option<&dyn TimingSink>,
    event_id: i32,
    year: i32,
) -> Result<CacheStatusResponse> {
    let in_memory = in_memory_status(event_id);
    let kv_key = ServerlessStorage::kv_scores_cache_key(event_id);
    let kv_text = storage
        .kv_get_optional_text(&kv_key)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let kv = match kv_text {
        Some(text) => match timed!(
            timing,
            "storage.kv_get_optional_parse_ms",
            parse_kv_scores_entry(&text)
        ) {
            Ok((_scores, remaining_ttl_seconds)) => CacheStatus {
                exists: true,
                remaining_ttl_seconds,
            },
            Err(_) => CacheStatus {
                exists: true,
                remaining_ttl_seconds: None,
            },
        },
        None => CacheStatus {
            exists: false,
            remaining_ttl_seconds: None,
        },
    };
    let r2_scores_key = ServerlessStorage::scores_key(event_id);
    let r2_exists = timed!(
        timing,
        "storage.r2_scores_exists_ms",
        storage
            .r2_key_exists(&r2_scores_key)
            .await
            .map_err(|e| worker::Error::RustError(e.to_string()))
    )?;
    let r2 = CacheStatus {
        exists: r2_exists,
        remaining_ttl_seconds: None,
    };
    Ok(CacheStatusResponse {
        event_id,
        year,
        in_memory,
        kv,
        r2,
        keys: CacheStatusKeys {
            kv: kv_key,
            r2_scores: r2_scores_key,
        },
    })
}

pub async fn admin_cache_status_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let instrumentation = request_instrumentation(&req, &ctx.env)?;
    let timing: Option<&dyn TimingSink> = Some(instrumentation.timing());
    let timing_rc: Option<Rc<dyn TimingSink>> = Some(instrumentation.timing_rc());
    let storage =
        timed!(timing, "storage.from_env_ms", storage_from_env(&ctx.env))?.with_timing(timing_rc);
    let query = timed!(
        timing,
        "request.parse_query_params_ms",
        parse_query_params(&req)
    )?;
    let event_id = match query
        .get("event")
        .and_then(|value| value.trim().parse::<i32>().ok())
    {
        Some(value) => value,
        None => return Response::error("event is required", 400),
    };
    let year = match query
        .get("yr")
        .and_then(|value| value.trim().parse::<i32>().ok())
    {
        Some(value) => value,
        None => return Response::error("yr is required", 400),
    };
    if let Some(response) = require_authorized(
        &instrumentation,
        &req,
        &ctx.env,
        &storage,
        timing,
        &query,
        event_id,
        year,
    )
    .await?
    {
        return Ok(response);
    }
    let response = build_cache_status_response(&storage, timing, event_id, year).await?;
    let details = serde_json::json!({
        "event_id": event_id,
        "year": year,
        "admin": true,
    });
    crate::finalize_resp!(
        instrumentation,
        &req,
        &ctx.env,
        details,
        Response::from_json(&response)
    )
}
