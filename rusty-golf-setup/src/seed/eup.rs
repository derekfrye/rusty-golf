use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub(crate) struct EupEvent {
    pub(crate) event: i64,
    pub(crate) name: String,
    pub(crate) score_view_step_factor: serde_json::Value,
    pub(crate) data_to_fill_if_event_and_year_missing: Vec<EupDataFill>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EupDataFill {
    pub(crate) golfers: Vec<EupGolfer>,
    pub(crate) event_user_player: Vec<EupEventUserPlayer>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EupGolfer {
    pub(crate) espn_id: i64,
    pub(crate) name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EupEventUserPlayer {
    pub(crate) bettor: String,
    pub(crate) golfer_espn_id: i64,
    pub(crate) score_view_step_factor: Option<serde_json::Value>,
}

pub(crate) fn load_events(eup_path: &Path, event_id_filter: Option<i64>) -> Result<Vec<EupEvent>> {
    if !eup_path.is_file() {
        bail!("missing file {}", eup_path.display());
    }
    let contents =
        fs::read_to_string(eup_path).with_context(|| format!("read {}", eup_path.display()))?;
    let mut events: Vec<EupEvent> = serde_json::from_str(&contents)
        .with_context(|| format!("parse {}", eup_path.display()))?;

    if let Some(event_id) = event_id_filter {
        events.retain(|event| event.event == event_id);
    }
    Ok(events)
}
