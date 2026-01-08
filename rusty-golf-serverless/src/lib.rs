#[cfg(target_arch = "wasm32")]
mod espn_client;
#[cfg(target_arch = "wasm32")]
pub mod storage;

pub use rusty_golf_core as core;

#[cfg(target_arch = "wasm32")]
use espn_client::ServerlessEspnClient;
#[cfg(target_arch = "wasm32")]
use storage::{AdminSeedRequest, EventListing, ServerlessStorage};
#[cfg(target_arch = "wasm32")]
use worker::{Env, Request, Response, Result, RouteContext, Router, event};
#[cfg(target_arch = "wasm32")]
use {
    rusty_golf_core::score::{
        cache_max_age_for_event, group_by_bettor_golfer_round, group_by_bettor_name_and_round,
        load_score_context, parse_score_request,
    },
    rusty_golf_core::view::index::{render_index_template, resolve_index_title_or_default},
    rusty_golf_core::view::score::{
        RefreshData, render_drop_down_bar_pure, render_line_score_tables,
        render_scores_template_pure, render_summary_scores,
        scores_and_last_refresh_to_line_score_tables,
    },
    serde::Deserialize,
    std::collections::HashMap,
};

#[cfg(target_arch = "wasm32")]
fn parse_query_params(req: &Request) -> Result<HashMap<String, String>> {
    let url = req
        .url()
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    Ok(url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect())
}

#[cfg(target_arch = "wasm32")]
fn respond_html(body: String) -> Result<Response> {
    let mut resp = Response::ok(body)?;
    resp.headers_mut()
        .set("Content-Type", "text/html")
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    Ok(resp)
}

#[cfg(target_arch = "wasm32")]
fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(target_arch = "wasm32")]
fn read_env_binding(env: &Env, var_name: &str) -> Result<String> {
    let value = env.var(var_name).map_err(|e| {
        worker::Error::RustError(format!(
            "Missing env var {var_name}; set it in wrangler.toml [vars]. {e}"
        ))
    })?;
    let value = value.to_string();
    if value.trim().is_empty() {
        Err(worker::Error::RustError(format!(
            "Env var {var_name} is empty; set it in wrangler.toml [vars]."
        )))
    } else {
        Ok(value)
    }
}

#[cfg(target_arch = "wasm32")]
fn storage_from_env(env: &Env) -> Result<ServerlessStorage> {
    let kv_binding = read_env_binding(env, "KV_BINDING")?;
    let r2_binding = read_env_binding(env, "R2_BINDING")?;
    ServerlessStorage::from_env(env, &kv_binding, &r2_binding).map_err(|e| {
        worker::Error::RustError(format!(
            "Storage binding error (KV_BINDING={kv_binding}, R2_BINDING={r2_binding}): {e}"
        ))
    })
}

#[cfg(target_arch = "wasm32")]
fn admin_enabled(env: &Env) -> bool {
    env.var("ADMIN_ENABLED")
        .ok()
        .map(|value| value.to_string() == "1")
        .unwrap_or(false)
}

#[cfg(target_arch = "wasm32")]
fn admin_token(env: &Env) -> Result<String> {
    let value = env
        .var("ADMIN_TOKEN")
        .map_err(|e| worker::Error::RustError(format!("Missing ADMIN_TOKEN env var: {e}")))?;
    let value = value.to_string();
    if value.trim().is_empty() {
        Err(worker::Error::RustError(
            "ADMIN_TOKEN is empty".to_string(),
        ))
    } else {
        Ok(value)
    }
}

#[cfg(target_arch = "wasm32")]
fn admin_request_token(req: &Request) -> Result<Option<String>> {
    if let Ok(Some(token)) = req.headers().get("x-admin-token") {
        if !token.trim().is_empty() {
            return Ok(Some(token));
        }
    }
    let query = parse_query_params(req)?;
    Ok(query.get("admin_token").cloned())
}

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct AdminCleanupRequest {
    event_id: i32,
    #[serde(default)]
    include_auth_tokens: bool,
}

#[cfg(target_arch = "wasm32")]
async fn admin_seed_handler(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
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

#[cfg(target_arch = "wasm32")]
async fn admin_cleanup_handler(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
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

#[cfg(target_arch = "wasm32")]
async fn scores_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let query = parse_query_params(&req)?;
    let score_req = match parse_score_request(&query) {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 400),
    };
    let storage = storage_from_env(&ctx.env)?;
    let cache_max_age = cache_max_age_for_event(&storage, score_req.event_id)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let espn_client = ServerlessEspnClient::new();
    let context = match load_score_context(
        &storage,
        &espn_client,
        score_req.event_id,
        score_req.year,
        score_req.use_cache,
        cache_max_age,
    )
    .await
    {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 500),
    };

    if score_req.want_json {
        Response::from_json(&context.data)
    } else {
        let bettor_struct = scores_and_last_refresh_to_line_score_tables(&context.from_db_scores);
        let markup = render_scores_template_pure(
            &context.data,
            score_req.expanded,
            &bettor_struct,
            context.global_step_factor,
            &context.player_step_factors,
            score_req.event_id,
            score_req.year,
            score_req.use_cache,
        );
        respond_html(markup.into_string())
    }
}

#[cfg(target_arch = "wasm32")]
async fn index_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let query = parse_query_params(&req)?;
    let event_str = query.get("event").map(String::as_str).unwrap_or("");
    let storage = storage_from_env(&ctx.env)?;
    let title = resolve_index_title_or_default(&storage, event_str).await;
    let markup = render_index_template(&title);
    respond_html(markup.into_string())
}

#[cfg(target_arch = "wasm32")]
async fn listing_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
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
    let body = render_listing(entries);
    respond_html(body)
}

#[cfg(target_arch = "wasm32")]
fn render_listing(entries: Vec<EventListing>) -> String {
    let mut body = String::from(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>KV Events</title></head><body>",
    );
    body.push_str("<h1>KV Events</h1>");
    if entries.is_empty() {
        body.push_str("<p>No events found.</p>");
    } else {
        body.push_str("<table>");
        body.push_str("<thead><tr><th>Event ID</th><th>Event Name</th><th>Step Factor</th><th>Refresh</th></tr></thead><tbody>");
        for entry in entries {
            body.push_str("<tr><td>");
            body.push_str(&entry.event_id.to_string());
            body.push_str("</td><td>");
            body.push_str(&escape_html(&entry.event_name));
            body.push_str("</td><td>");
            body.push_str(&entry.score_view_step_factor.to_string());
            body.push_str("</td><td>");
            body.push_str(&entry.refresh_from_espn.to_string());
            body.push_str("</td></tr>");
        }
        body.push_str("</tbody></table>");
    }
    body.push_str("</body></html>");
    body
}

#[cfg(target_arch = "wasm32")]
async fn static_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let assets = ctx.env.assets("ASSETS")?;
    let mut url = req
        .url()
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let path = url.path();
    let stripped = path.strip_prefix("/static/").unwrap_or(path);
    let rewritten = format!("/{}", stripped.trim_start_matches('/'));
    url.set_path(&rewritten);
    assets.fetch(url.to_string(), None).await
}

#[cfg(target_arch = "wasm32")]
async fn scores_summary_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let query = parse_query_params(&req)?;
    let score_req = match parse_score_request(&query) {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 400),
    };
    if !score_req.expanded {
        let resp = Response::empty()?.with_status(204);
        return Ok(resp);
    }
    let storage = storage_from_env(&ctx.env)?;
    let cache_max_age = cache_max_age_for_event(&storage, score_req.event_id)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let espn_client = ServerlessEspnClient::new();
    let context = match load_score_context(
        &storage,
        &espn_client,
        score_req.event_id,
        score_req.year,
        score_req.use_cache,
        cache_max_age,
    )
    .await
    {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 500),
    };
    let summary = group_by_bettor_name_and_round(&context.data.score_struct);
    let markup = render_summary_scores(&summary);
    respond_html(markup.into_string())
}

#[cfg(target_arch = "wasm32")]
async fn scores_chart_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let query = parse_query_params(&req)?;
    let score_req = match parse_score_request(&query) {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 400),
    };
    let storage = storage_from_env(&ctx.env)?;
    let cache_max_age = cache_max_age_for_event(&storage, score_req.event_id)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let espn_client = ServerlessEspnClient::new();
    let context = match load_score_context(
        &storage,
        &espn_client,
        score_req.event_id,
        score_req.year,
        score_req.use_cache,
        cache_max_age,
    )
    .await
    {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 500),
    };
    let summary_scores_x = group_by_bettor_name_and_round(&context.data.score_struct);
    let detailed_scores = group_by_bettor_golfer_round(&context.data.score_struct);
    let markup = render_drop_down_bar_pure(
        &summary_scores_x,
        &detailed_scores,
        context.global_step_factor,
        &context.player_step_factors,
    );
    respond_html(markup.into_string())
}

#[cfg(target_arch = "wasm32")]
async fn scores_linescore_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let query = parse_query_params(&req)?;
    let score_req = match parse_score_request(&query) {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 400),
    };
    let storage = storage_from_env(&ctx.env)?;
    let cache_max_age = cache_max_age_for_event(&storage, score_req.event_id)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let espn_client = ServerlessEspnClient::new();
    let context = match load_score_context(
        &storage,
        &espn_client,
        score_req.event_id,
        score_req.year,
        score_req.use_cache,
        cache_max_age,
    )
    .await
    {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 500),
    };
    let bettor_struct = scores_and_last_refresh_to_line_score_tables(&context.from_db_scores);
    let refresh_data = RefreshData {
        last_refresh: context.data.last_refresh.clone(),
        last_refresh_source: context.data.last_refresh_source.clone(),
    };
    let markup = render_line_score_tables(&bettor_struct, &refresh_data);
    respond_html(markup.into_string())
}

#[cfg(target_arch = "wasm32")]
#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    let router = Router::new();

    let result = router
        .get_async("/", |req, ctx| async move { index_handler(req, ctx).await })
        .get_async("/static/*path", |req, ctx| async move {
            static_handler(req, ctx).await
        })
        .get_async("/health", |_, ctx| async move {
            let _storage = storage_from_env(&ctx.env)?;
            Response::ok("ok")
        })
        .get_async("/scores", |req, ctx| async move {
            scores_handler(req, ctx).await
        })
        .get_async("/listing", |req, ctx| async move {
            listing_handler(req, ctx).await
        })
        .get_async("/scores/summary", |req, ctx| async move {
            scores_summary_handler(req, ctx).await
        })
        .get_async("/scores/chart", |req, ctx| async move {
            scores_chart_handler(req, ctx).await
        })
        .get_async("/scores/linescore", |req, ctx| async move {
            scores_linescore_handler(req, ctx).await
        })
        .post_async("/admin/seed", |req, ctx| async move {
            admin_seed_handler(req, ctx).await
        })
        .post_async("/admin/cleanup", |req, ctx| async move {
            admin_cleanup_handler(req, ctx).await
        })
        .run(req, env)
        .await;

    match result {
        Ok(resp) => Ok(resp),
        Err(err) => Response::error(err.to_string(), 500),
    }
}
