use crate::espn::{EspnClient, HttpEspnClient};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

mod bettors;
mod cache;
mod eup;
mod events;
mod golfers;

pub(crate) use bettors::{
    bettors_selection_exists, ensure_list_bettors, load_bettors_selection,
    persist_bettors_selection,
};
pub(crate) use cache::{
    has_cached_events, load_cached_golfers, load_event_golfers,
};
pub(crate) use eup::{eup_event_exists, load_eup_json};
pub(crate) use events::{ensure_list_events, print_list_event_error};
pub(crate) use golfers::{
    output_json_path, set_golfers_by_bettor, take_golfers_by_bettor,
};

pub(crate) struct ReplState {
    pub(crate) cached_events: Option<Vec<(String, String)>>,
    pub(crate) cached_bettors: Option<Vec<String>>,
    pub(crate) golfers_by_bettor: Option<Vec<GolferSelection>>,
    pub(crate) eup_json_path: Option<PathBuf>,
    pub(crate) event_cache_dir: PathBuf,
    pub(crate) bettors_selection_path: PathBuf,
    pub(crate) output_json_path: Option<PathBuf>,
    pub(crate) espn: Arc<dyn EspnClient>,
    pub(crate) _temp_dir: TempDir,
}

impl ReplState {
    pub(crate) fn new(
        eup_json_path: Option<PathBuf>,
        output_json_path: Option<PathBuf>,
    ) -> Result<Self> {
        Self::new_with_client(
            eup_json_path,
            output_json_path,
            Arc::new(HttpEspnClient),
        )
    }

    pub(crate) fn new_with_client(
        eup_json_path: Option<PathBuf>,
        output_json_path: Option<PathBuf>,
        espn: Arc<dyn EspnClient>,
    ) -> Result<Self> {
        let temp_dir = TempDir::new().context("create event cache dir")?;
        let event_cache_dir = temp_dir.path().join("espn_events");
        let bettors_selection_path = temp_dir.path().join("bettors.txt");
        fs::create_dir_all(&event_cache_dir)
            .with_context(|| format!("create {}", event_cache_dir.display()))?;
        Ok(Self {
            cached_events: None,
            cached_bettors: None,
            golfers_by_bettor: None,
            eup_json_path,
            event_cache_dir,
            bettors_selection_path,
            output_json_path,
            espn,
            _temp_dir: temp_dir,
        })
    }
}

#[derive(Clone)]
pub(crate) struct GolferSelection {
    pub(crate) bettor: String,
    pub(crate) golfer_espn_id: i64,
}
