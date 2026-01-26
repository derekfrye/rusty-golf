#![cfg(target_arch = "wasm32")]

use worker::{AnalyticsEngineDataPointBuilder, Env, Request, Result};

use super::TimingEntry;

pub(super) fn slow_log_threshold_ms(env: &Env) -> Option<f64> {
    let secret = env.secret("LOGGING_TOTAL_MS").ok()?.to_string();
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<f64>().ok()
}

pub(super) fn emit_analytics(
    req: &Request,
    env: &Env,
    details: &serde_json::Value,
    total_ms: f64,
    phases: Vec<TimingEntry>,
    emit_full: bool,
) -> Result<()> {
    let dataset = match env.analytics_engine("REQUEST_METRICS") {
        Ok(dataset) => dataset,
        Err(_) => return Ok(()),
    };
    let path = req
        .url()
        .map(|url| url.path().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let method = req.method().to_string();
    let request_id = req
        .headers()
        .get("cf-ray")
        .ok()
        .flatten()
        .unwrap_or_default();
    let event_id = details
        .get("event_id")
        .and_then(|value| value.as_i64())
        .map(|value| value.to_string())
        .unwrap_or_default();
    let year = details
        .get("year")
        .and_then(|value| value.as_i64())
        .map(|value| value.to_string())
        .unwrap_or_default();

    let total_blobs = vec![
        "total".to_string(),
        "".to_string(),
        method.clone(),
        event_id.clone(),
        year.clone(),
        request_id.clone(),
    ];
    let _ = AnalyticsEngineDataPointBuilder::new()
        .indexes([path.as_str()])
        .add_double(total_ms)
        .blobs(total_blobs)
        .write_to(&dataset);

    if emit_full {
        for phase in phases {
            let phase_blobs = vec![
                "phase".to_string(),
                phase.name.clone(),
                method.clone(),
                event_id.clone(),
                year.clone(),
                request_id.clone(),
            ];
            let _ = AnalyticsEngineDataPointBuilder::new()
                .indexes([path.as_str()])
                .add_double(phase.ms)
                .blobs(phase_blobs)
                .write_to(&dataset);
        }
    }
    Ok(())
}
