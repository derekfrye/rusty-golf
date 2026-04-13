#![cfg(target_arch = "wasm32")]

use serde::Serialize;
use worker::{Response, Result, RouteContext};

#[derive(Serialize)]
struct VersionResponse {
    git_sha: &'static str,
    git_sha_short: &'static str,
    package_version: &'static str,
}

pub async fn version_handler(_ctx: RouteContext<()>) -> Result<Response> {
    let mut resp = Response::from_json(&VersionResponse {
        git_sha: env!("RUSTY_GOLF_GIT_SHA"),
        git_sha_short: env!("RUSTY_GOLF_GIT_SHA_SHORT"),
        package_version: env!("CARGO_PKG_VERSION"),
    })?;
    resp.headers_mut()
        .set("cache-control", "no-store, max-age=0")?;
    Ok(resp)
}
