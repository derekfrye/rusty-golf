#![cfg(target_arch = "wasm32")]

use std::collections::HashMap;

use worker::{Env, Request, Response, Result};

use crate::storage::ServerlessStorage;

pub fn parse_query_params(req: &Request) -> Result<HashMap<String, String>> {
    let url = req
        .url()
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    Ok(url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect())
}

pub fn respond_html(body: String) -> Result<Response> {
    let mut resp = Response::ok(body)?;
    resp.headers_mut()
        .set("Content-Type", "text/html")
        .map_err(|e| worker::Error::RustError(e.to_string()))?;
    Ok(resp)
}

// pub fn escape_html(value: &str) -> String {
//     value
//         .replace('&', "&amp;")
//         .replace('<', "&lt;")
//         .replace('>', "&gt;")
//         .replace('"', "&quot;")
//         .replace('\'', "&#39;")
// }

pub fn read_env_binding(env: &Env, var_name: &str) -> Result<String> {
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

pub fn storage_from_env(env: &Env) -> Result<ServerlessStorage> {
    let kv_binding = read_env_binding(env, "KV_BINDING")?;
    let r2_binding = read_env_binding(env, "R2_BINDING")?;
    ServerlessStorage::from_env(env, &kv_binding, &r2_binding).map_err(|e| {
        worker::Error::RustError(format!(
            "Storage binding error (KV_BINDING={kv_binding}, R2_BINDING={r2_binding}): {e}"
        ))
    })
}
