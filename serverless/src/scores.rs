#![cfg(target_arch = "wasm32")]

use worker::{Request, Response, Result, RouteContext};

use rusty_golf_core::score::{
    cache_max_age_for_event, group_by_bettor_golfer_round, group_by_bettor_name_and_round,
    load_score_context, parse_score_request,
};
use rusty_golf_core::view::score::{
    RefreshData, render_drop_down_bar_pure, render_line_score_tables, render_scores_template_pure,
    render_summary_scores, scores_and_last_refresh_to_line_score_tables,
};

use crate::espn_client::ServerlessEspnClient;
use crate::utils::{parse_query_params, respond_html, storage_from_env};

fn parse_score_request_from_req(req: &Request) -> Result<rusty_golf_core::score::ScoreRequest> {
    let query = parse_query_params(req)?;
    parse_score_request(&query).map_err(|e| worker::Error::RustError(e.to_string()))
}

async fn load_context(
    score_req: &rusty_golf_core::score::ScoreRequest,
    ctx: &RouteContext<()>,
) -> Result<rusty_golf_core::score::ScoreContext> {
    let storage = storage_from_env(&ctx.env)?;
    let cache_max_age = cache_max_age_for_event(&storage, score_req.event_id)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    let espn_client = ServerlessEspnClient::new(storage.clone());
    let context = load_score_context(
        &storage,
        &espn_client,
        score_req.event_id,
        score_req.year,
        score_req.use_cache,
        cache_max_age,
    )
    .await
    .map_err(|e| worker::Error::RustError(e.to_string()))?;
    Ok(context)
}

pub async fn scores_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let score_req = match parse_score_request_from_req(&req) {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 400),
    };
    let context = load_context(&score_req, &ctx).await?;

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

pub async fn scores_summary_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let score_req = match parse_score_request_from_req(&req) {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 400),
    };
    if !score_req.expanded {
        let resp = Response::empty()?.with_status(204);
        return Ok(resp);
    }
    let context = load_context(&score_req, &ctx).await?;
    let summary = group_by_bettor_name_and_round(&context.data.score_struct);
    let markup = render_summary_scores(&summary);
    respond_html(markup.into_string())
}

pub async fn scores_chart_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let score_req = match parse_score_request_from_req(&req) {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 400),
    };
    let context = load_context(&score_req, &ctx).await?;
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

pub async fn scores_linescore_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let score_req = match parse_score_request_from_req(&req) {
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(), 400),
    };
    let context = load_context(&score_req, &ctx).await?;
    let bettor_struct = scores_and_last_refresh_to_line_score_tables(&context.from_db_scores);
    let refresh_data = RefreshData {
        last_refresh: context.data.last_refresh.clone(),
        last_refresh_source: context.data.last_refresh_source.clone(),
    };
    let markup = render_line_score_tables(&bettor_struct, &refresh_data);
    respond_html(markup.into_string())
}
