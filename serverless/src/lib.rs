#[cfg(target_arch = "wasm32")]
mod admin;
#[cfg(target_arch = "wasm32")]
mod espn_client;
#[cfg(target_arch = "wasm32")]
mod index;
#[cfg(target_arch = "wasm32")]
mod listing;
#[cfg(target_arch = "wasm32")]
mod scores;
#[cfg(target_arch = "wasm32")]
mod static_assets;
#[cfg(target_arch = "wasm32")]
pub mod storage;
#[cfg(target_arch = "wasm32")]
mod utils;

pub use rusty_golf_core as core;

#[cfg(target_arch = "wasm32")]
use admin::{
    admin_cleanup_handler, admin_end_date_handler, admin_seed_handler, admin_test_lock_handler,
    admin_test_unlock_handler,
};
#[cfg(target_arch = "wasm32")]
use index::index_handler;
#[cfg(target_arch = "wasm32")]
use listing::listing_handler;
#[cfg(target_arch = "wasm32")]
use scores::{
    scores_chart_handler, scores_handler, scores_linescore_handler, scores_summary_handler,
};
#[cfg(target_arch = "wasm32")]
use static_assets::static_handler;
#[cfg(target_arch = "wasm32")]
use utils::storage_from_env;
#[cfg(target_arch = "wasm32")]
use worker::{Env, Request, Response, Result, Router, event};

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
        .post_async("/admin/event_end_date", |req, ctx| async move {
            admin_end_date_handler(req, ctx).await
        })
        .post_async("/admin/test_lock", |req, ctx| async move {
            admin_test_lock_handler(req, ctx).await
        })
        .post_async("/admin/test_unlock", |req, ctx| async move {
            admin_test_unlock_handler(req, ctx).await
        })
        .run(req, env)
        .await;

    match result {
        Ok(resp) => Ok(resp),
        Err(err) => Response::error(err.to_string(), 500),
    }
}
