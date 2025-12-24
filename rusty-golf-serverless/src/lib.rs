#[cfg(target_arch = "wasm32")]
pub mod storage;
#[cfg(target_arch = "wasm32")]
mod espn_client;

pub use rusty_golf_core as core;

#[cfg(target_arch = "wasm32")]
use espn_client::ServerlessEspnClient;
#[cfg(target_arch = "wasm32")]
use storage::ServerlessStorage;
#[cfg(target_arch = "wasm32")]
use worker::{event, Env, Request, Response, Result, RouteContext, Router};
#[cfg(target_arch = "wasm32")]
use {
    rusty_golf_core::score::{
        cache_max_age_for_event, load_score_context, parse_score_request,
        group_by_bettor_golfer_round, group_by_bettor_name_and_round,
    },
    rusty_golf_core::view::score::{
        render_drop_down_bar_pure, render_line_score_tables, render_scores_template_pure,
        render_summary_scores, scores_and_last_refresh_to_line_score_tables, RefreshData,
    },
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
async fn scores_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let query = parse_query_params(&req)?;
    let score_req = match parse_score_request(&query) {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 400),
    };
    let storage = ServerlessStorage::from_env(
        &ctx.env,
        ServerlessStorage::KV_BINDING,
        ServerlessStorage::R2_BINDING,
    )
    .map_err(|e| worker::Error::RustError(e.to_string()))?;
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
        let bettor_struct =
            scores_and_last_refresh_to_line_score_tables(&context.from_db_scores);
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
async fn scores_summary_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let query = parse_query_params(&req)?;
    let score_req = match parse_score_request(&query) {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 400),
    };
    if !score_req.expanded {
        let mut resp = Response::empty()?;
        resp.set_status(204);
        return Ok(resp);
    }
    let storage = ServerlessStorage::from_env(
        &ctx.env,
        ServerlessStorage::KV_BINDING,
        ServerlessStorage::R2_BINDING,
    )
    .map_err(|e| worker::Error::RustError(e.to_string()))?;
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
    let storage = ServerlessStorage::from_env(
        &ctx.env,
        ServerlessStorage::KV_BINDING,
        ServerlessStorage::R2_BINDING,
    )
    .map_err(|e| worker::Error::RustError(e.to_string()))?;
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
    let storage = ServerlessStorage::from_env(
        &ctx.env,
        ServerlessStorage::KV_BINDING,
        ServerlessStorage::R2_BINDING,
    )
    .map_err(|e| worker::Error::RustError(e.to_string()))?;
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

    router
        .get("/health", |_, ctx| async move {
            let _storage = ServerlessStorage::from_env(
                &ctx.env,
                ServerlessStorage::KV_BINDING,
                ServerlessStorage::R2_BINDING,
            )
            .map_err(|e| worker::Error::RustError(e.to_string()))?;
            Response::ok("ok")
        })
        .get("/scores", |req, ctx| async move { scores_handler(req, ctx).await })
        .get("/scores/summary", |req, ctx| async move {
            scores_summary_handler(req, ctx).await
        })
        .get("/scores/chart", |req, ctx| async move {
            scores_chart_handler(req, ctx).await
        })
        .get("/scores/linescore", |req, ctx| async move {
            scores_linescore_handler(req, ctx).await
        })
        .run(req, env)
        .await
}
