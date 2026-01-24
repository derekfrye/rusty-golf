pub trait TimingSink {
    fn record(&self, name: &'static str, ms: f64);
}

#[derive(Clone, Copy)]
pub struct TimingStart {
    #[cfg(target_arch = "wasm32")]
    start_ms: f64,
    #[cfg(not(target_arch = "wasm32"))]
    start: std::time::Instant,
}

#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn start_timing() -> TimingStart {
    TimingStart {
        start_ms: js_sys::Date::now(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn start_timing() -> TimingStart {
    TimingStart {
        start: std::time::Instant::now(),
    }
}

#[must_use]
pub fn elapsed_ms(start: &TimingStart) -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() - start.start_ms
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        start.start.elapsed().as_secs_f64() * 1000.0
    }
}

#[macro_export]
macro_rules! timed {
    ($timing:expr, $name:expr, $expr:expr) => {{
        let start = $crate::timing::start_timing();
        let result = $expr;
        $crate::timing::record_timing($timing, $name, start);
        result
    }};
}

pub fn record_timing(timing: Option<&dyn TimingSink>, name: &'static str, start: TimingStart) {
    if let Some(timing) = timing {
        timing.record(name, elapsed_ms(&start));
    }
}
