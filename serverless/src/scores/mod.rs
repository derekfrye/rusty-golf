#![cfg(target_arch = "wasm32")]

use worker::{Request, Result};

use rusty_golf_core::score::{
    cache_max_age_for_event, load_score_context_with_timing, parse_score_request,
};
use rusty_golf_core::timed;
use rusty_golf_core::timing::TimingSink;

use crate::espn_client::ServerlessEspnClient;
use crate::utils::parse_query_params;

mod chart_handler;
mod linescore_handler;
mod scores_handler;
mod summary_handler;

pub use chart_handler::scores_chart_handler;
pub use linescore_handler::scores_linescore_handler;
pub use scores_handler::scores_handler;
pub use summary_handler::scores_summary_handler;

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
