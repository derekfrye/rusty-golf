use super::types::EupEventInput;
use rusty_golf_core::model::Scores;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct EspnFixture {
    score_struct: Vec<Scores>,
}

pub fn load_eup_event(
    workspace_root: &Path,
    event_id: i64,
) -> Result<EupEventInput, Box<dyn Error>> {
    let path = workspace_root.join("tests/tests/test05_dbprefill.json");
    let contents = fs::read_to_string(path)?;
    let events: Vec<EupEventInput> = serde_json::from_str(&contents)?;
    events
        .into_iter()
        .find(|event| event.event == event_id)
        .ok_or_else(|| format!("Missing event {event_id} in test05_dbprefill.json").into())
}

pub fn load_score_struct(workspace_root: &Path) -> Result<Vec<Scores>, Box<dyn Error>> {
    let path = workspace_root.join("tests/tests/test03_espn_json_responses.json");
    let contents = fs::read_to_string(path)?;
    let fixture: EspnFixture = serde_json::from_str(&contents)?;
    Ok(fixture.score_struct)
}

pub fn load_espn_cache(workspace_root: &Path) -> Result<serde_json::Value, Box<dyn Error>> {
    let path = workspace_root.join("tests/tests/test03_espn_json_responses.json");
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}
