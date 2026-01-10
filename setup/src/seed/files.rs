use crate::seed::eup::EupEvent;
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize)]
struct EventDetails<'a> {
    event_name: &'a str,
    score_view_step_factor: &'a serde_json::Value,
    refresh_from_espn: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_date: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct GolferOut<'a> {
    eup_id: i64,
    espn_id: i64,
    golfer_name: &'a str,
    bettor_name: &'a str,
    group: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    score_view_step_factor: Option<&'a serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct PlayerFactor<'a> {
    golfer_espn_id: i64,
    bettor_name: &'a str,
    step_factor: &'a serde_json::Value,
}

#[derive(Debug, Serialize)]
struct AuthTokensDoc<'a> {
    tokens: &'a [String],
}

#[derive(Debug, Serialize)]
struct SeededAtDoc {
    seeded_at: String,
}

pub(crate) fn write_event_files(
    event: &EupEvent,
    refresh_from_espn: i64,
    end_date: Option<&str>,
    root: &Path,
) -> Result<()> {
    let data_to_fill = event
        .data_to_fill_if_event_and_year_missing
        .first()
        .ok_or_else(|| {
            anyhow!(
                "no data_to_fill_if_event_and_year_missing for {}",
                event.event
            )
        })?;

    let event_dir = root.join(event.event.to_string());
    fs::create_dir_all(&event_dir).with_context(|| format!("create {}", event_dir.display()))?;

    let details = EventDetails {
        event_name: &event.name,
        score_view_step_factor: &event.score_view_step_factor,
        refresh_from_espn,
        end_date,
    };
    write_json(&event_dir.join("event_details.json"), &details)?;

    let mut golfers_by_id = HashMap::new();
    for golfer in &data_to_fill.golfers {
        golfers_by_id.insert(golfer.espn_id, golfer.name.as_str());
    }

    let mut bettor_counts: HashMap<&str, usize> = HashMap::new();
    let mut golfers_out = Vec::new();
    let mut eup_id = 1_i64;
    for entry in &data_to_fill.event_user_player {
        let count = bettor_counts.entry(entry.bettor.as_str()).or_insert(0);
        *count += 1;

        let golfer_name = golfers_by_id.get(&entry.golfer_espn_id).ok_or_else(|| {
            anyhow!(
                "missing golfer_espn_id {} in golfers list for event {}",
                entry.golfer_espn_id,
                event.event
            )
        })?;

        golfers_out.push(GolferOut {
            eup_id,
            espn_id: entry.golfer_espn_id,
            golfer_name,
            bettor_name: entry.bettor.as_str(),
            group: *count,
            score_view_step_factor: entry.score_view_step_factor.as_ref(),
        });
        eup_id += 1;
    }

    write_json(&event_dir.join("golfers.json"), &golfers_out)?;

    let player_factors: Vec<PlayerFactor<'_>> = data_to_fill
        .event_user_player
        .iter()
        .filter_map(|entry| {
            entry
                .score_view_step_factor
                .as_ref()
                .map(|factor| PlayerFactor {
                    golfer_espn_id: entry.golfer_espn_id,
                    bettor_name: entry.bettor.as_str(),
                    step_factor: factor,
                })
        })
        .collect();
    write_json(&event_dir.join("player_factors.json"), &player_factors)?;

    let seeded_at = SeededAtDoc {
        seeded_at: Utc::now().to_rfc3339(),
    };
    write_json(&event_dir.join("seeded_at.json"), &seeded_at)?;

    Ok(())
}

pub(crate) fn write_auth_tokens(tokens: &[String], event_dir: &Path) -> Result<()> {
    let payload = AuthTokensDoc { tokens };
    write_json(&event_dir.join("auth_tokens.json"), &payload)?;
    Ok(())
}

fn write_json<T: Serialize>(path: &Path, data: &T) -> Result<()> {
    let file = fs::File::create(path).with_context(|| format!("create {}", path.display()))?;
    serde_json::to_writer(&file, data).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}
