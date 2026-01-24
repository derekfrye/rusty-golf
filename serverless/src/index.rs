#![cfg(target_arch = "wasm32")]

use std::collections::HashMap;

use maud::Markup;

use worker::{Request, Response, Result, RouteContext};

use rusty_golf_core::score::{
    cache_max_age_for_event, load_score_context, parse_score_request,
};
use rusty_golf_core::view::index::{
    render_index_template_with_scores, resolve_index_title_or_default,
};
use rusty_golf_core::view::score::{
    render_scores_template_pure, scores_and_last_refresh_to_line_score_tables,
};

use crate::espn_client::ServerlessEspnClient;
use crate::storage::ServerlessStorage;
use crate::utils::{parse_query_params, respond_html, storage_from_env};

async fn try_render_scores_markup(
    query: &HashMap<String, String>,
    storage: &ServerlessStorage,
) -> Option<Markup> {
    let score_req = parse_score_request(query).ok()?;
    let cache_max_age = cache_max_age_for_event(storage, score_req.event_id)
        .await
        .ok()?;
    let espn_client = ServerlessEspnClient::new(storage.clone());
    let context = load_score_context(
        storage,
        &espn_client,
        score_req.event_id,
        score_req.year,
        score_req.use_cache,
        cache_max_age,
    )
    .await
    .ok()?;
    let bettor_struct = scores_and_last_refresh_to_line_score_tables(&context.from_db_scores);
    Some(render_scores_template_pure(
        &context.data,
        score_req.expanded,
        &bettor_struct,
        context.global_step_factor,
        &context.player_step_factors,
        score_req.event_id,
        score_req.year,
        score_req.use_cache,
    ))
}

pub async fn index_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let query = parse_query_params(&req)?;
    let event_str = query.get("event").map(String::as_str).unwrap_or("");
    let storage = storage_from_env(&ctx.env)?;
    let title = resolve_index_title_or_default(&storage, event_str).await;
    let scores_markup = if query.contains_key("event")
        && query.contains_key("yr")
        && matches!(query.get("nojs").map(String::as_str), Some("1"))
    {
        try_render_scores_markup(&query, &storage).await
    } else {
        None
    };
    let markup = render_index_template_with_scores(&title, scores_markup);
    respond_html(markup.into_string())
}
