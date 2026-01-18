use super::AppMode;
use super::cli::{Cli, FileConfig};
use super::parse::parse_event_ids;
use anyhow::{Result, anyhow};

pub(crate) fn build_get_event_details_mode(cli: &Cli, file_config: &FileConfig) -> Result<AppMode> {
    let one_shot = if cli.one_shot {
        true
    } else {
        file_config.one_shot.unwrap_or(false)
    };
    if !one_shot {
        return Err(anyhow!("--one-shot is required for --mode=get_event_details"));
    }

    let output_json = cli
        .output_json
        .clone()
        .or_else(|| file_config.output_json.clone());
    let output_json_stdout = cli.output_json_stdout || file_config.output_json_stdout.unwrap_or(false);
    if output_json.is_none() && !output_json_stdout {
        return Err(anyhow!(
            "missing --output-json or --output-json-stdout for --mode=get_event_details"
        ));
    }

    let event_id_input = resolve_event_id_input(cli, file_config);
    let event_ids = event_id_input
        .as_deref()
        .map(parse_event_ids)
        .transpose()?;

    Ok(AppMode::GetEventDetails {
        eup_json: cli.eup_json.clone().or_else(|| file_config.eup_json.clone()),
        output_json,
        output_json_stdout,
        event_ids,
    })
}

fn resolve_event_id_input(cli: &Cli, file_config: &FileConfig) -> Option<String> {
    cli.event_id.clone().or_else(|| {
        file_config
            .event_id
            .as_ref()
            .map(super::cli::EventIdConfig::as_string)
    })
}
