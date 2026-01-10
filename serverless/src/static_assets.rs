#![cfg(target_arch = "wasm32")]

use worker::{Request, Response, Result, RouteContext};

pub async fn static_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
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
