use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

const SCOREBOARD_HEADER_URL: &str = "https://site.web.api.espn.com/apis/v2/scoreboard/header?sport=golf&league=pga&region=us&lang=en&contentorigin=espn";

#[derive(Debug, Deserialize)]
struct ScoreboardHeader {
    sports: Vec<Sport>,
}

#[derive(Debug, Deserialize)]
struct Sport {
    leagues: Vec<League>,
}

#[derive(Debug, Deserialize)]
struct League {
    events: Vec<HeaderEvent>,
}

#[derive(Debug, Deserialize)]
struct HeaderEvent {
    id: String,
    #[serde(rename = "endDate")]
    end_date: Option<String>,
}

pub(crate) fn fetch_end_dates() -> Result<HashMap<i64, String>> {
    let response = reqwest::blocking::get(SCOREBOARD_HEADER_URL)
        .context("fetch scoreboard header")?;
    let header: ScoreboardHeader = response
        .json()
        .context("parse scoreboard header")?;

    let mut end_dates = HashMap::new();
    for sport in header.sports {
        for league in sport.leagues {
            for event in league.events {
                if let Some(end_date) = event.end_date
                    && let Ok(event_id) = event.id.parse::<i64>()
                {
                    end_dates.insert(event_id, end_date);
                }
            }
        }
    }

    Ok(end_dates)
}
