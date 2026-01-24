#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::rc::Rc;

use serde::Serialize;
use worker::{Env, Request, Result, console_log};

use rusty_golf_core::timing::{TimingSink, TimingStart, elapsed_ms, start_timing};

#[derive(Clone, Serialize)]
struct TimingEntry {
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

    pub fn log_request(&self, req: &Request, details: serde_json::Value) -> Result<()> {
        let url = req
            .url()
            .map_err(|e| worker::Error::RustError(e.to_string()))?;
        let phases = self.entries.borrow().clone();
        let log = serde_json::json!({
            "type": "instrumentation",
            "method": req.method().to_string(),
            "path": url.path(),
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

pub fn instrumentation_from_request(req: &Request, env: &Env) -> Result<Option<Rc<TimingCollector>>> {
    let secret = match env.secret("INSTRUMENT_TOKEN") {
        Ok(secret) => secret.to_string(),
        Err(_) => return Ok(None),
    };
    if secret.trim().is_empty() {
        return Ok(None);
    }
    let provided = match req.headers().get("x-instrument-token")? {
        Some(value) => value,
        None => return Ok(None),
    };
    if provided.trim().is_empty() {
        return Ok(None);
    }
    if provided == secret {
        Ok(Some(Rc::new(TimingCollector::new())))
    } else {
        Ok(None)
    }
}
