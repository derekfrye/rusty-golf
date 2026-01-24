#![cfg(target_arch = "wasm32")]

use worker::{Request, Response, Result, RouteContext};

use rusty_golf_core::score::{
    cache_max_age_for_event, group_by_bettor_golfer_round, group_by_bettor_name_and_round,
    load_score_context_with_timing, parse_score_request,
};
use rusty_golf_core::timed;
use rusty_golf_core::timing::TimingSink;
use rusty_golf_core::view::score::{
    RefreshData, render_drop_down_bar_pure, render_line_score_tables, render_scores_template_pure,
    render_summary_scores, scores_and_last_refresh_to_line_score_tables,
};
use std::rc::Rc;

use crate::espn_client::ServerlessEspnClient;
use crate::instrument::request_instrumentation;
use crate::utils::{parse_query_params, respond_html, storage_from_env};

fn parse_score_request_from_req(req: &Request) -> Result<rusty_golf_core::score::ScoreRequest> {
    let query = parse_query_params(req)?;
    parse_score_request(&query).map_err(|e| worker::Error::RustError(e.to_string()))
}

async fn load_context(
    score_req: &rusty_golf_core::score::ScoreRequest,
    storage: &crate::storage::ServerlessStorage,
    timing: Option<&dyn TimingSink>,
) -> Result<rusty_golf_core::score::ScoreContext> {
    let cache_max_age = timed!(
        timing,
        "cache.max_age_ms",
        cache_max_age_for_event(storage, score_req.event_id)
            .await
            .map_err(|e| worker::Error::RustError(e.to_string()))
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
        .map_err(|e| worker::Error::RustError(e.to_string()))
    )?;
    Ok(context)
}

pub async fn scores_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let instrumentation = request_instrumentation(&req, &ctx.env)?;
    let timing: Option<&dyn TimingSink> = Some(instrumentation.timing());
    let timing_rc: Option<Rc<dyn TimingSink>> = Some(instrumentation.timing_rc());
    let storage = timed!(timing, "storage.from_env_ms", storage_from_env(&ctx.env))?
        .with_timing(timing_rc);
    let score_req = match timed!(
        timing,
        "request.parse_score_request_ms",
        parse_score_request_from_req(&req)
    ) {
        Ok(value) => value,
        Err(err) => {
            let details = serde_json::json!({
                "status": 400,
            });
            return crate::finalize_resp!(
                instrumentation,
                &req,
                &ctx.env,
                details,
                Response::error(err.to_string(), 400)
            );
        }
    };
    let context = load_context(&score_req, &storage, timing).await?;

    if score_req.want_json {
        let resp = timed!(
            timing,
            "response.json_ms",
            Response::from_json(&context.data)
        );
        let details = serde_json::json!({
            "event_id": score_req.event_id,
            "year": score_req.year,
            "cache": score_req.use_cache,
            "json": true,
            "expanded": score_req.expanded,
            "cache_hit": context.data.cache_hit,
        });
        crate::finalize_resp!(
            instrumentation,
            &req,
            &ctx.env,
            details,
            resp
        )
    } else {
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
        let resp = timed!(timing, "response.html_ms", respond_html(markup.into_string()));
        let details = serde_json::json!({
            "event_id": score_req.event_id,
            "year": score_req.year,
            "cache": score_req.use_cache,
            "json": false,
            "expanded": score_req.expanded,
            "cache_hit": context.data.cache_hit,
        });
        crate::finalize_resp!(
            instrumentation,
            &req,
            &ctx.env,
            details,
            resp
        )
    }
}

pub async fn scores_summary_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let instrumentation = request_instrumentation(&req, &ctx.env)?;
    let timing: Option<&dyn TimingSink> = Some(instrumentation.timing());
    let timing_rc: Option<Rc<dyn TimingSink>> = Some(instrumentation.timing_rc());
    let storage = timed!(timing, "storage.from_env_ms", storage_from_env(&ctx.env))?
        .with_timing(timing_rc);
    let score_req = match timed!(
        timing,
        "request.parse_score_request_ms",
        parse_score_request_from_req(&req)
    ) {
        Ok(value) => value,
        Err(err) => {
            let details = serde_json::json!({
                "status": 400,
            });
            return crate::finalize_resp!(
                instrumentation,
                &req,
                &ctx.env,
                details,
                Response::error(err.to_string(), 400)
            );
        }
    };
    if !score_req.expanded {
        let details = serde_json::json!({
            "event_id": score_req.event_id,
            "year": score_req.year,
            "cache": score_req.use_cache,
            "json": false,
            "expanded": score_req.expanded,
            "cache_hit": null,
            "status": 204,
        });
        return crate::finalize_resp!(
            instrumentation,
            &req,
            &ctx.env,
            details,
            Response::empty().map(|response| response.with_status(204))
        );
    }
    let context = load_context(&score_req, &storage, timing).await?;
    let summary = timed!(
        timing,
        "view.group_summary_scores_ms",
        group_by_bettor_name_and_round(&context.data.score_struct)
    );
    let markup = timed!(timing, "view.render_summary_ms", render_summary_scores(&summary));
    let resp = timed!(timing, "response.html_ms", respond_html(markup.into_string()));
    let details = serde_json::json!({
        "event_id": score_req.event_id,
        "year": score_req.year,
        "cache": score_req.use_cache,
        "json": false,
        "expanded": score_req.expanded,
        "cache_hit": context.data.cache_hit,
    });
    crate::finalize_resp!(
        instrumentation,
        &req,
        &ctx.env,
        details,
        resp
    )
}

pub async fn scores_chart_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let instrumentation = request_instrumentation(&req, &ctx.env)?;
    let timing: Option<&dyn TimingSink> = Some(instrumentation.timing());
    let timing_rc: Option<Rc<dyn TimingSink>> = Some(instrumentation.timing_rc());
    let storage = timed!(timing, "storage.from_env_ms", storage_from_env(&ctx.env))?
        .with_timing(timing_rc);
    let score_req = match timed!(
        timing,
        "request.parse_score_request_ms",
        parse_score_request_from_req(&req)
    ) {
        Ok(value) => value,
        Err(err) => {
            let details = serde_json::json!({
                "status": 400,
            });
            return crate::finalize_resp!(
                instrumentation,
                &req,
                &ctx.env,
                details,
                Response::error(err.to_string(), 400)
            );
        }
    };
    let context = load_context(&score_req, &storage, timing).await?;
    let (summary_scores_x, detailed_scores) = timed!(
        timing,
        "view.group_chart_scores_ms",
        (
            group_by_bettor_name_and_round(&context.data.score_struct),
            group_by_bettor_golfer_round(&context.data.score_struct),
        )
    );
    let markup = timed!(
        timing,
        "view.render_chart_ms",
        render_drop_down_bar_pure(
            &summary_scores_x,
            &detailed_scores,
            context.global_step_factor,
            &context.player_step_factors,
        )
    );
    let resp = timed!(timing, "response.html_ms", respond_html(markup.into_string()));
    let details = serde_json::json!({
        "event_id": score_req.event_id,
        "year": score_req.year,
        "cache": score_req.use_cache,
        "json": false,
        "expanded": score_req.expanded,
        "cache_hit": context.data.cache_hit,
    });
    crate::finalize_resp!(
        instrumentation,
        &req,
        &ctx.env,
        details,
        resp
    )
}

pub async fn scores_linescore_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let instrumentation = request_instrumentation(&req, &ctx.env)?;
    let timing: Option<&dyn TimingSink> = Some(instrumentation.timing());
    let timing_rc: Option<Rc<dyn TimingSink>> = Some(instrumentation.timing_rc());
    let storage = timed!(timing, "storage.from_env_ms", storage_from_env(&ctx.env))?
        .with_timing(timing_rc);
    let score_req = match timed!(
        timing,
        "request.parse_score_request_ms",
        parse_score_request_from_req(&req)
    ) {
        Ok(value) => value,
        Err(err) => {
            let details = serde_json::json!({
                "status": 400,
            });
            return crate::finalize_resp!(
                instrumentation,
                &req,
                &ctx.env,
                details,
                Response::error(err.to_string(), 400)
            );
        }
    };
    let context = load_context(&score_req, &storage, timing).await?;
    let bettor_struct = timed!(
        timing,
        "view.build_linescore_tables_ms",
        scores_and_last_refresh_to_line_score_tables(&context.from_db_scores)
    );
    let refresh_data = RefreshData {
        last_refresh: context.data.last_refresh.clone(),
        last_refresh_source: context.data.last_refresh_source.clone(),
    };
    let markup = timed!(
        timing,
        "view.render_linescore_ms",
        render_line_score_tables(&bettor_struct, &refresh_data)
    );
    let resp = timed!(timing, "response.html_ms", respond_html(markup.into_string()));
    let details = serde_json::json!({
        "event_id": score_req.event_id,
        "year": score_req.year,
        "cache": score_req.use_cache,
        "json": false,
        "expanded": score_req.expanded,
        "cache_hit": context.data.cache_hit,
    });
    crate::finalize_resp!(
        instrumentation,
        &req,
        &ctx.env,
        details,
        resp
    )
}
