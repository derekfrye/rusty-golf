use super::AppMode;
use super::cli::{Cli, FileConfig, GolfersByBettorConfig};
use super::parse::parse_golfers_by_bettor;
use anyhow::{Result, anyhow};

pub(crate) fn build_new_event_mode(cli: &Cli, file_config: &FileConfig) -> Result<AppMode> {
    let one_shot = if cli.one_shot {
        true
    } else {
        file_config.one_shot.unwrap_or(false)
    };
    let output_json = cli
        .output_json
        .clone()
        .or_else(|| file_config.output_json.clone());
    let event_id = cli.event_id.or(file_config.event_id);
    let golfers_by_bettor = resolve_golfers_by_bettor(cli, file_config)?;

    if one_shot {
        if event_id.is_none() {
            return Err(anyhow!("missing --event-id for --one-shot"));
        }
        if output_json.is_none() {
            return Err(anyhow!("missing --output-json for --one-shot"));
        }
        if golfers_by_bettor.is_none() {
            return Err(anyhow!("missing --golfers-by-bettor for --one-shot"));
        }
    }

    Ok(AppMode::NewEvent {
        eup_json: cli.eup_json.clone().or_else(|| file_config.eup_json.clone()),
        output_json,
        one_shot,
        event_id,
        golfers_by_bettor,
    })
}

fn resolve_golfers_by_bettor(
    cli: &Cli,
    file_config: &FileConfig,
) -> Result<Option<Vec<super::GolferByBettorInput>>> {
    if let Some(value) = cli.golfers_by_bettor.as_ref() {
        return Ok(Some(parse_golfers_by_bettor(value)?));
    }
    if let Some(value) = file_config.golfers_by_bettor.as_ref() {
        let entries = match value {
            GolfersByBettorConfig::Json(raw) => parse_golfers_by_bettor(raw)?,
            GolfersByBettorConfig::Entries(entries) => entries.clone(),
        };
        return Ok(Some(entries));
    }
    Ok(None)
}
