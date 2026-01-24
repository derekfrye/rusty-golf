#![cfg(target_arch = "wasm32")]

use worker::{Env, Request, Response, Result};

use crate::utils::parse_query_params;

pub fn admin_auth_response(req: &Request, env: &Env) -> Result<Option<Response>> {
    if !admin_enabled(env) {
        return Ok(Some(Response::error("not found", 404)?));
    }
    let expected = admin_token(env)?;
    let Some(provided) = admin_request_token(req)? else {
        return Ok(Some(Response::error("unauthorized", 401)?));
    };
    if provided != expected {
        return Ok(Some(Response::error("unauthorized", 401)?));
    }
    Ok(None)
}

fn admin_enabled(env: &Env) -> bool {
    env.var("ADMIN_ENABLED")
        .ok()
        .map(|value| value.to_string() == "1")
        .unwrap_or(false)
}

fn admin_token(env: &Env) -> Result<String> {
    let value = env
        .var("ADMIN_TOKEN")
        .map_err(|e| worker::Error::RustError(format!("Missing ADMIN_TOKEN env var: {e}")))?;
    let value = value.to_string();
    if value.trim().is_empty() {
        Err(worker::Error::RustError("ADMIN_TOKEN is empty".to_string()))
    } else {
        Ok(value)
    }
}

fn admin_request_token(req: &Request) -> Result<Option<String>> {
    if let Ok(Some(token)) = req.headers().get("x-admin-token") {
        if !token.trim().is_empty() {
            return Ok(Some(token));
        }
    }
    let query = parse_query_params(req)?;
    Ok(query.get("admin_token").cloned())
}
