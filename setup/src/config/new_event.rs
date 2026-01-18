use super::AppMode;
use super::cli::{Cli, FileConfig, GolfersByBettorConfig};
use super::parse::{parse_golfers_by_bettor, parse_single_event_id};
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
    let output_json_stdout =
        (cli.output_json_stdout || file_config.output_json_stdout.unwrap_or(false)) && one_shot;
    let event_id_input = resolve_event_id_input(cli, file_config);
    let event_id = event_id_input
        .as_deref()
        .map(parse_single_event_id)
        .transpose()?;
    let golfers_by_bettor = resolve_golfers_by_bettor(cli, file_config)?;

    if one_shot {
        if event_id.is_none() {
            return Err(anyhow!("missing --event-id for --one-shot"));
        }
        if output_json.is_none() && !output_json_stdout {
            return Err(anyhow!(
                "missing --output-json or --output-json-stdout for --one-shot"
            ));
        }
        if golfers_by_bettor.is_none() {
            return Err(anyhow!("missing --golfers-by-bettor for --one-shot"));
        }
    }

    Ok(AppMode::NewEvent {
        eup_json: cli
            .eup_json
            .clone()
            .or_else(|| file_config.eup_json.clone()),
        output_json,
        output_json_stdout,
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

fn resolve_event_id_input(cli: &Cli, file_config: &FileConfig) -> Option<String> {
    cli.event_id.clone().or_else(|| {
        file_config
            .event_id
            .as_ref()
            .map(super::cli::EventIdConfig::as_string)
    })
}
