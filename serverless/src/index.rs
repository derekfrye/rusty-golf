#![cfg(target_arch = "wasm32")]

use std::collections::HashMap;

use maud::Markup;

use worker::{Request, Response, Result, RouteContext};

use rusty_golf_core::score::{
    cache_max_age_for_event, load_score_context_with_timing, parse_score_request,
};
use rusty_golf_core::timed;
use rusty_golf_core::timing::TimingSink;
use rusty_golf_core::view::index::{
    render_index_template_with_scores, resolve_index_title_or_default,
};
use rusty_golf_core::view::score::{
    render_scores_template_pure, scores_and_last_refresh_to_line_score_tables,
};
use std::rc::Rc;

use crate::espn_client::ServerlessEspnClient;
use crate::instrument::request_instrumentation;
use crate::storage::ServerlessStorage;
use crate::utils::{parse_query_params, respond_html, storage_from_env};

async fn try_render_scores_markup(
    query: &HashMap<String, String>,
    storage: &ServerlessStorage,
    timing: Option<&dyn TimingSink>,
) -> Option<Markup> {
    let score_req = timed!(
        timing,
        "request.parse_score_request_ms",
        parse_score_request(query).ok()
    )?;
    let cache_max_age = timed!(
        timing,
        "cache.max_age_ms",
        cache_max_age_for_event(storage, score_req.event_id).await.ok()
    )?;
    let espn_client = ServerlessEspnClient::new(storage.clone());
    let context = timed!(
        timing,
        "score_context.load_ms",
        load_score_context_with_timing(
            storage,
            &espn_client,
            score_req.event_id,
            score_req.year,
            score_req.use_cache,
            cache_max_age,
            timing,
        )
        .await
        .ok()
    )?;
    let bettor_struct = timed!(
        timing,
        "view.build_linescore_tables_ms",
        scores_and_last_refresh_to_line_score_tables(&context.from_db_scores)
    );
    let markup = timed!(
        timing,
        "view.render_scores_ms",
        render_scores_template_pure(
            &context.data,
            score_req.expanded,
            &bettor_struct,
            context.global_step_factor,
            &context.player_step_factors,
            score_req.event_id,
            score_req.year,
            score_req.use_cache,
        )
    );
    Some(markup)
}

pub async fn index_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let instrumentation = request_instrumentation(&req, &ctx.env)?;
    let timing: Option<&dyn TimingSink> = Some(instrumentation.timing());
    let timing_rc: Option<Rc<dyn TimingSink>> = Some(instrumentation.timing_rc());
    let query = parse_query_params(&req)?;
    let event_str = query.get("event").map(String::as_str).unwrap_or("");
    let storage = timed!(timing, "storage.from_env_ms", storage_from_env(&ctx.env))?
        .with_timing(timing_rc);
    let title = timed!(
        timing,
        "view.resolve_index_title_ms",
        resolve_index_title_or_default(&storage, event_str).await
    );
    let scores_markup = if query.contains_key("event")
        && query.contains_key("yr")
        && matches!(query.get("nojs").map(String::as_str), Some("1"))
    {
        try_render_scores_markup(&query, &storage, timing).await
    } else {
        None
    };
    let markup = timed!(
        timing,
        "view.render_index_ms",
        render_index_template_with_scores(&title, scores_markup)
    );
    let resp = timed!(timing, "response.html_ms", respond_html(markup.into_string()));
    let details = serde_json::json!({
        "event_id": query.get("event").and_then(|value| value.parse::<i32>().ok()),
        "year": query.get("yr").and_then(|value| value.parse::<i32>().ok()),
        "nojs": matches!(query.get("nojs").map(String::as_str), Some("1")),
    });
    crate::finalize_resp!(
        instrumentation,
        &req,
        &ctx.env,
        details,
        resp
    )
}
