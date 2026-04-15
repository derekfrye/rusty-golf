#![cfg(target_arch = "wasm32")]

use std::rc::Rc;
use worker::{Request, Response, Result, RouteContext};

use rusty_golf_core::score::group_by_bettor_name_and_round;
use rusty_golf_core::timed;
use rusty_golf_core::timing::TimingSink;
use rusty_golf_core::view::score::render_summary_scores;

use crate::instrument::request_instrumentation;
use crate::utils::{respond_html, storage_from_env};

use super::{load_context, parse_score_request_from_req};

pub async fn scores_summary_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let instrumentation = request_instrumentation(&req, &ctx.env)?;
    let timing: Option<&dyn TimingSink> = Some(instrumentation.timing());
    let timing_rc: Option<Rc<dyn TimingSink>> = Some(instrumentation.timing_rc());
    let storage =
        timed!(timing, "storage.from_env_ms", storage_from_env(&ctx.env))?.with_timing(timing_rc);
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
    let markup = timed!(
        timing,
        "view.render_summary_ms",
        render_summary_scores(&summary)
    );
    let resp = timed!(
        timing,
        "response.html_ms",
        respond_html(markup.into_string())
    );
    let details = serde_json::json!({
        "event_id": score_req.event_id,
        "year": score_req.year,
        "cache": score_req.use_cache,
        "json": false,
        "expanded": score_req.expanded,
        "cache_hit": context.data.cache_hit,
    });
    crate::finalize_resp!(instrumentation, &req, &ctx.env, details, resp)
}
