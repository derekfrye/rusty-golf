#![cfg(target_arch = "wasm32")]

use maud::{Markup, html};
use serde::Serialize;
use worker::{Request, Response, Result, RouteContext};
use std::rc::Rc;

mod admin;

use crate::storage::EventListing;
use crate::instrument::request_instrumentation;
use rusty_golf_core::timed;
use rusty_golf_core::timing::TimingSink;
use crate::utils::{parse_query_params, respond_html, storage_from_env};

#[derive(Serialize)]
pub(super) struct AdminListingResponse {
    events: Vec<EventListing>,
    kv_keys: Vec<String>,
    r2_keys: Vec<String>,
    event_id: Option<i32>,
    scores_exists: Option<bool>,
    espn_cache_exists: Option<bool>,
}

pub async fn listing_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let instrumentation = request_instrumentation(&req, &ctx.env)?;
    let timing: Option<&dyn TimingSink> = Some(instrumentation.timing());
    let timing_rc: Option<Rc<dyn TimingSink>> = Some(instrumentation.timing_rc());
    let storage = timed!(timing, "storage.from_env_ms", storage_from_env(&ctx.env))?
        .with_timing(timing_rc);
    if let Some(response) =
        admin::try_admin_listing_response(&req, &ctx.env, &instrumentation, timing, &storage)
            .await?
    {
        return Ok(response);
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
