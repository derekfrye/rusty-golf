#![cfg(target_arch = "wasm32")]

use maud::{Markup, html};
use worker::{Request, Response, Result, RouteContext};

use crate::storage::EventListing;
use crate::utils::{parse_query_params, respond_html, storage_from_env};

pub async fn listing_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let query = parse_query_params(&req)?;
    let auth_token = match query.get("auth_token") {
        Some(value) if !value.trim().is_empty() => value.trim(),
        _ => return Response::error("auth_token is required", 401),
    };
    let storage = storage_from_env(&ctx.env)?;
    if !storage
        .auth_token_valid(auth_token)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?
    {
        return Response::error("auth_token is invalid", 401);
    }

    let entries = storage
        .list_event_listings()
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let markup = render_listing(entries);
    respond_html(markup.into_string())
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
                                th { "Step Factor" }
                                th { "Refresh" }
                            }
                        }
                        tbody {
                            @for entry in entries {
                                tr {
                                    td { (entry.event_id) }
                                    td { (entry.event_name) }
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
