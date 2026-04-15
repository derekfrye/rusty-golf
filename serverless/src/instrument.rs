#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use worker::{Env, Request, Response, Result, console_log};

use rusty_golf_core::timing::{TimingSink, TimingStart, elapsed_ms, start_timing};

mod analytics;
use analytics::{emit_analytics, slow_log_threshold_ms};

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[macro_export]
macro_rules! finalize_resp {
    ($instr:expr, $req:expr, $env:expr, $details:expr, $resp:expr) => {{
        let response = $resp?;
        $instr.finalize_response($req, $env, $details, response)
    }};
}

#[derive(Clone, Serialize)]
pub(super) struct TimingEntry {
    name: String,
    ms: f64,
}

pub struct TimingCollector {
    started_at: TimingStart,
    entries: RefCell<Vec<TimingEntry>>,
}

impl TimingCollector {
    #[must_use]
    pub fn new() -> Self {
        Self {
            started_at: start_timing(),
            entries: RefCell::new(Vec::new()),
        }
    }

    #[must_use]
    pub fn total_ms(&self) -> f64 {
        elapsed_ms(&self.started_at)
    }

    #[must_use]
    pub fn entries(&self) -> Vec<TimingEntry> {
        self.entries.borrow().clone()
    }

    pub fn log_request(&self, req: &Request, details: serde_json::Value) -> Result<()> {
        let url = req
            .url()
            .map_err(|e| worker::Error::RustError(e.to_string()))?;
        let request_id = req.headers().get("cf-ray")?.unwrap_or_default();
        let phases = self.entries.borrow().clone();
        let log = serde_json::json!({
            "type": "instrumentation",
            "method": req.method().to_string(),
            "path": url.path(),
            "request_id": request_id,
            "total_ms": elapsed_ms(&self.started_at),
            "phases": phases,
            "details": details,
        });
        console_log!("{}", log);
        Ok(())
    }
}

impl TimingSink for TimingCollector {
    fn record(&self, name: &'static str, ms: f64) {
        self.entries.borrow_mut().push(TimingEntry {
            name: name.to_string(),
            ms,
        });
    }
}

pub struct RequestInstrumentation {
    collector: Rc<TimingCollector>,
    log_console: bool,
}

impl RequestInstrumentation {
    #[must_use]
    pub fn timing(&self) -> &dyn TimingSink {
        self.collector.as_ref()
    }

    #[must_use]
    pub fn timing_rc(&self) -> Rc<dyn TimingSink> {
        self.collector.clone() as Rc<dyn TimingSink>
    }

    #[must_use]
    pub fn instrument_header_valid(&self) -> bool {
        self.log_console
    }

    pub fn finalize(&self, req: &Request, env: &Env, details: serde_json::Value) -> Result<()> {
        let total_ms = self.collector.total_ms();
        let slow_threshold = slow_log_threshold_ms(env);
        let is_slow = slow_threshold
            .map(|threshold| total_ms >= threshold)
            .unwrap_or(false);
        let emit_sampled = should_emit_sampled_request(env)?;
        let emit_full = self.log_console || is_slow || emit_sampled;
        if self.log_console || is_slow {
            self.collector.log_request(req, details.clone())?;
        }
        let phases = if emit_full {
            self.collector.entries()
        } else {
            Vec::new()
        };
        emit_analytics(req, env, &details, total_ms, phases, emit_full)?;
        Ok(())
    }

    pub fn finalize_response(
        &self,
        req: &Request,
        env: &Env,
        details: serde_json::Value,
        response: Response,
    ) -> Result<Response> {
        self.finalize(req, env, details)?;
        Ok(response)
    }
}

pub fn request_instrumentation(req: &Request, env: &Env) -> Result<RequestInstrumentation> {
    let log_console = has_valid_instrument_token(req, env)?;
    Ok(RequestInstrumentation {
        collector: Rc::new(TimingCollector::new()),
        log_console,
    })
}

fn should_emit_sampled_request(env: &Env) -> Result<bool> {
    let value = match env.var("LOG_EVERY_N_REQUESTS") {
        Ok(value) => value.to_string(),
        Err(_) => return Ok(false),
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(false);
    }
    let every_n: u64 = match trimmed.parse() {
        Ok(value) if value > 0 => value,
        _ => return Ok(false),
    };
    let count = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    Ok(count % every_n == 0)
}

fn has_valid_instrument_token(req: &Request, env: &Env) -> Result<bool> {
    let secret = match env.secret("INSTRUMENT_TOKEN") {
        Ok(secret) => secret.to_string(),
        Err(_) => return Ok(false),
    };
    let secret_trimmed = secret.trim();
    if secret_trimmed.is_empty() {
        return Ok(false);
    }
    let provided = match req.headers().get("x-instrument-token")? {
        Some(value) => value,
        None => return Ok(false),
    };
    let provided_trimmed = provided.trim();
    if provided_trimmed.is_empty() {
        return Ok(false);
    }
    Ok(provided_trimmed == secret_trimmed)
}
