#![cfg(target_arch = "wasm32")]

use worker::{Request, Response, Result, RouteContext};

use rusty_golf_core::view::index::{render_index_template, resolve_index_title_or_default};

use crate::utils::{parse_query_params, respond_html, storage_from_env};

pub async fn index_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let query = parse_query_params(&req)?;
    let event_str = query.get("event").map(String::as_str).unwrap_or("");
    let storage = storage_from_env(&ctx.env)?;
    let title = resolve_index_title_or_default(&storage, event_str).await;
    let markup = render_index_template(&title);
    respond_html(markup.into_string())
}
