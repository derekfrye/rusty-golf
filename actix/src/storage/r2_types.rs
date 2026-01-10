use serde::{Deserialize, Serialize};

use super::R2Storage;

#[derive(Debug, Serialize, Deserialize)]
pub struct R2EventDetails {
    pub event_name: String,
    pub score_view_step_factor: f32,
    pub refresh_from_espn: i64,
    pub end_date: Option<String>,
}

impl R2Storage {
    pub(crate) fn scores_key(event_id: i32) -> String {
        format!("events/{event_id}/scores.json")
    }

    pub(crate) fn golfers_key(event_id: i32) -> String {
        format!("events/{event_id}/golfers.json")
    }

    pub(crate) fn event_key(event_id: i32) -> String {
        format!("events/{event_id}/event.json")
    }
}
