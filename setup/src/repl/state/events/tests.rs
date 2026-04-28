use super::{EventListMode, ensure_list_events};
use crate::espn::EspnClient;
use crate::repl::state::ReplState;
use anyhow::Result;
use indicatif::ProgressBar;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct TestEspnClient {
    list_calls: AtomicUsize,
}

impl TestEspnClient {
    fn new() -> Self {
        Self {
            list_calls: AtomicUsize::new(0),
        }
    }

    fn list_calls(&self) -> usize {
        self.list_calls.load(Ordering::SeqCst)
    }
}

impl EspnClient for TestEspnClient {
    fn list_events(&self) -> Result<Vec<(String, String)>> {
        self.list_calls.fetch_add(1, Ordering::SeqCst);
        Ok(vec![("101".to_string(), "ESPN Event".to_string())])
    }

    fn fetch_event_name(&self, _event_id: i64, _cache_dir: &Path) -> Result<String> {
        Ok("Fetched Event".to_string())
    }

    fn fetch_event_names_parallel(
        &self,
        _event_ids: &[i64],
        _cache_dir: &Path,
        _progress: Option<&ProgressBar>,
    ) -> Vec<(i64, String)> {
        Vec::new()
    }

    fn fetch_event_json_cached(&self, _event_id: i64, _cache_dir: &Path) -> Result<Value> {
        Ok(Value::Null)
    }

    fn fetch_scoreboard_header_cached(&self, _cache_dir: &Path) -> Result<Value> {
        Ok(Value::Null)
    }
}

#[test]
fn refresh_espn_reuses_cached_kv_without_reloading_it() {
    let client = Arc::new(TestEspnClient::new());
    let mut state = ReplState::new_with_client(None, None, None, client.clone()).unwrap();
    state.cached_kv_events = Some(vec![("202".to_string(), "KV Event".to_string())]);

    let events = ensure_list_events(&mut state, EventListMode::RefreshEspn, false).unwrap();

    assert_eq!(client.list_calls(), 1);
    assert_eq!(
        events,
        vec![
            ("101".to_string(), "ESPN Event".to_string()),
            ("202".to_string(), "KV Event".to_string())
        ]
    );
}

#[test]
fn refresh_kv_reuses_cached_espn_without_reloading_it() {
    let client = Arc::new(TestEspnClient::new());
    let mut state = ReplState::new_with_client(None, None, None, client.clone()).unwrap();
    state.cached_espn_events = Some(vec![("101".to_string(), "ESPN Event".to_string())]);

    let events = ensure_list_events(&mut state, EventListMode::RefreshKv, false).unwrap();

    assert_eq!(client.list_calls(), 0);
    assert_eq!(events, vec![("101".to_string(), "ESPN Event".to_string())]);
}
