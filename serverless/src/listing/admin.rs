#![cfg(target_arch = "wasm32")]

use worker::{Env, Request, Response, Result};

use crate::instrument::RequestInstrumentation;
use crate::storage::ServerlessStorage;
use crate::listing::AdminListingResponse;
use rusty_golf_core::timed;
use rusty_golf_core::timing::TimingSink;

fn admin_header_valid(req: &Request, env: &Env) -> Result<bool> {
    let enabled = env
        .var("ADMIN_ENABLED")
        .ok()
        .map(|value| value.to_string() == "1")
        .unwrap_or(false);
    if !enabled {
        return Ok(false);
    }
    let expected = match env.var("ADMIN_TOKEN") {
        Ok(value) => value.to_string(),
        Err(_) => return Ok(false),
    };
    let Some(provided) = req.headers().get("x-admin-token")? else {
        return Ok(false);
    };
    if provided.trim().is_empty() {
        return Ok(false);
    }
    Ok(provided == expected)
}

pub async fn try_admin_listing_response(
    req: &Request,
    env: &Env,
    instrumentation: &RequestInstrumentation,
    timing: Option<&dyn TimingSink>,
    storage: &ServerlessStorage,
) -> Result<Option<Response>> {
    if !admin_header_valid(req, env)? {
        return Ok(None);
    }

    let query = timed!(
        timing,
        "request.parse_query_params_ms",
        crate::utils::parse_query_params(req)
    )?;
    let event_id = query
        .get("event_id")
        .and_then(|value| value.trim().parse::<i32>().ok());
    let events = timed!(
        timing,
        "storage.list_event_listings_ms",
        storage
            .list_event_listings()
            .await
            .map_err(|e| worker::Error::RustError(e.to_string()))
    )?;
    let kv_keys = timed!(
        timing,
        "storage.kv_list_keys_ms",
        storage
            .kv_list_keys_with_prefix("")
            .await
            .map_err(|e| worker::Error::RustError(e.to_string()))
    )?;
    let (r2_keys, scores_exists, espn_cache_exists) = if let Some(id) = event_id {
        let scores_key = ServerlessStorage::scores_key(id);
        let cache_key = ServerlessStorage::espn_cache_key(id);
        let scores_exists = timed!(
            timing,
            "storage.r2_scores_exists_ms",
            storage
                .r2_key_exists(&scores_key)
                .await
                .map_err(|e| worker::Error::RustError(e.to_string()))
        )?;
        let espn_cache_exists = timed!(
            timing,
            "storage.r2_espn_cache_exists_ms",
            storage
                .r2_key_exists(&cache_key)
                .await
                .map_err(|e| worker::Error::RustError(e.to_string()))
        )?;
        (Vec::new(), Some(scores_exists), Some(espn_cache_exists))
    } else {
        let r2_keys = timed!(
            timing,
            "storage.r2_list_keys_ms",
            storage
                .r2_list_keys_with_prefix(None)
                .await
                .map_err(|e| worker::Error::RustError(e.to_string()))
        )?;
        (r2_keys, None, None)
    };
    let resp = timed!(
        timing,
        "response.json_ms",
        Response::from_json(&AdminListingResponse {
            events,
            kv_keys,
            r2_keys,
            event_id,
            scores_exists,
            espn_cache_exists,
        })
    );
    let details = serde_json::json!({
        "admin": true,
        "event_id": event_id,
    });
    let response = crate::finalize_resp!(instrumentation, req, env, details, resp)?;
    Ok(Some(response))
}
