#![cfg(target_arch = "wasm32")]

use maud::{Markup, html};
use serde::Serialize;
use worker::{Env, Request, Response, Result, RouteContext};
use std::rc::Rc;

use crate::storage::EventListing;
use crate::instrument::request_instrumentation;
use rusty_golf_core::timed;
use rusty_golf_core::timing::TimingSink;
use crate::utils::{parse_query_params, respond_html, storage_from_env};

#[derive(Serialize)]
struct AdminListingResponse {
    events: Vec<EventListing>,
    kv_keys: Vec<String>,
    r2_keys: Vec<String>,
    event_id: Option<i32>,
    scores_exists: Option<bool>,
    espn_cache_exists: Option<bool>,
}

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

pub async fn listing_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let instrumentation = request_instrumentation(&req, &ctx.env)?;
    let timing: Option<&dyn TimingSink> = Some(instrumentation.timing());
    let timing_rc: Option<Rc<dyn TimingSink>> = Some(instrumentation.timing_rc());
    let storage = timed!(timing, "storage.from_env_ms", storage_from_env(&ctx.env))?
        .with_timing(timing_rc);
    if admin_header_valid(&req, &ctx.env)? {
        let query = timed!(
            timing,
            "request.parse_query_params_ms",
            parse_query_params(&req)
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
            let scores_key = crate::storage::ServerlessStorage::scores_key(id);
            let cache_key = crate::storage::ServerlessStorage::espn_cache_key(id);
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
        return crate::finalize_resp!(
            instrumentation,
            &req,
            &ctx.env,
            details,
            resp
        );
    }

    let query = timed!(
        timing,
        "request.parse_query_params_ms",
        parse_query_params(&req)
    )?;
    let auth_token = match query.get("auth_token") {
        Some(value) if !value.trim().is_empty() => value.trim(),
        _ => {
            let details = serde_json::json!({
                "admin": false,
                "status": 401,
            });
            return crate::finalize_resp!(
                instrumentation,
                &req,
                &ctx.env,
                details,
                Response::error("auth_token is required", 401)
            );
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
        let details = serde_json::json!({
            "admin": false,
            "status": 401,
        });
        return crate::finalize_resp!(
            instrumentation,
            &req,
            &ctx.env,
            details,
            Response::error("auth_token is invalid", 401)
        );
    }

    let entries = timed!(
        timing,
        "storage.list_event_listings_ms",
        storage
            .list_event_listings()
            .await
            .map_err(|e| worker::Error::RustError(e.to_string()))
    )?;
    let markup = timed!(timing, "view.render_listing_ms", render_listing(entries));
    let resp = timed!(timing, "response.html_ms", respond_html(markup.into_string()));
    let details = serde_json::json!({
        "admin": false,
    });
    crate::finalize_resp!(
        instrumentation,
        &req,
        &ctx.env,
        details,
        resp
    )
}

fn render_listing(entries: Vec<EventListing>) -> Markup {
    html! {
        (maud::DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title { "KV Events" }
            }
            body {
                h1 { "KV Events" }
                @if entries.is_empty() {
                    p { "No events found." }
                } @else {
                    table {
                        thead {
                            tr {
                                th { "Event ID" }
                                th { "Event Name" }
                                th { "Link" }
                                th { "Step Factor" }
                                th { "Refresh" }
                            }
                        }
                        tbody {
                            @for entry in entries {
                                @let link = entry.year.map(|year| format!("/?event={}&yr={}", entry.event_id, year));
                                tr {
                                    td { (entry.event_id) }
                                    td { (entry.event_name) }
                                    td {
                                        @if let Some(url) = link {
                                            a href=(url) { "Open" }
                                        } @else {
                                            "n/a"
                                        }
                                    }
                                    td { (entry.score_view_step_factor) }
                                    td { (entry.refresh_from_espn) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
