#[cfg(target_arch = "wasm32")]
pub mod storage;

pub use rusty_golf_core as core;

#[cfg(target_arch = "wasm32")]
use storage::ServerlessStorage;
#[cfg(target_arch = "wasm32")]
use worker::{event, Env, Request, Response, Result, Router};

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
        .run(req, env)
        .await
}
