use serde_json::Value;
use std::{fs, path::PathBuf};


/// # Errors
///
/// Will return `Err` if the file is not readable
pub fn check_readable_file(file: &str) -> Result<String, String> {
    // split by semi-colon
    let files = file.split(';');
    for file in files {
        let path = PathBuf::from(file);
        if !path.is_file() || fs::metadata(&path).is_err() {
            return Err(format!("The sql startup script '{file}' is not readable."));
        }
    }
    Ok(file.to_string())
}

/// # Errors
///
/// Will return `Err` if the file is not readable or is not valid json
///
/// # Panics
///
/// Will panic if the file is not found or the json is not in the correct format
pub fn check_readable_file_and_json(file: &str) -> Result<Value, String> {
    let path = PathBuf::from(file);
    if !path.is_file() || fs::metadata(&path).is_err() {
        return Err(format!("The json file '{file}' is not readable."));
    }
    let contents = fs::read_to_string(&path).unwrap();
    let json: Value = serde_json::from_str(&contents).unwrap();
    validate_json_format(&json)?;
    Ok(json)
}

/// Validate the json file format
/// format we expect is this:
/// [{ "event": <int>, "year": <int>, "name":"value", "score_view_step_factor": <float>, "data_to_fill_if_event_and_year_missing": [
/// { "bettors": [{"PlayerName", "PlayerName2", "PlayerName3"...}]
/// , "golfers": [{"name": "Firstname Lastname", "espn_id": <int>}, {"name": "Firstname Lastname", "espn_id": <int>}, ...]
/// , "event_user_player": [{"bettor": "PlayerName", "golfer_espn_id": <int>}, {"bettor": "PlayerName", "golfer_espn_id": <int>}, ...]
/// }]}]
///
/// # Errors
///
/// Will return `Err` if the json is not in the correct format
///
/// # Panics
///
/// Will panic if the json is not in the correct format
fn validate_json_format(json: &Value) -> Result<(), String> {
    if !json.is_array() {
        return Err("The json file is not in the correct format.".to_string());
    }

    // check the json against this format
    let expected_keys = vec![
        "event",
        "year",
        "name",
        "data_to_fill_if_event_and_year_missing",
        "score_view_step_factor",
    ];
    for element in json.as_array().unwrap() {
        for key in element.as_object().unwrap().keys() {
            if !expected_keys.contains(&key.as_str()) {
                return Err(format!(
                    "The json file is not in the correct format. Expected keys: {expected_keys:?}"
                ));
            }
            let event = &element["event"];
            if !event.is_number() {
                return Err(
                    "The json key event is not in the correct format. Expected a number."
                        .to_string(),
                );
            }
            let year = &element["year"];
            if !year.is_number() {
                return Err(
                    "The json key year is not in the correct format. Expected a number."
                        .to_string(),
                );
            }
            let name = &element["name"];
            if !name.is_string() {
                return Err(
                    "The json key name is not in the correct format. Expected a string."
                        .to_string(),
                );
            }
            let score_view_step_factor = &element["score_view_step_factor"];
            if !score_view_step_factor.is_number() {
                return Err(
                    "The json key score_view_step_factor is not in the correct format. Expected a number.".to_string()
                );
            }
        }

        // now check the data_to_fill_if_event_and_year_missing
        let data_to_fill = element["data_to_fill_if_event_and_year_missing"]
            .as_array()
            .unwrap();
        for data in data_to_fill {
            let expected_keys = vec!["bettors", "golfers", "event_user_player"];
            for (key, _) in data.as_object().unwrap() {
                if !expected_keys.contains(&key.as_str()) {
                    return Err(format!(
                        "The json key data_to_fill_if_event_and_year_missing is not in the correct format. Expected keys: {expected_keys:?}"
                    ));
                }
            }
        }

        let bettors_check = data_to_fill
            .iter()
            .flat_map(|x| x["bettors"].as_array().unwrap())
            .collect::<Vec<_>>();
        // check bettors are just strings
        for bettor in bettors_check {
            if !bettor.is_string() {
                return Err(
                    "The json key bettors is not in the correct format. Expected strings."
                        .to_string(),
                );
            }
        }

        let golfers_check = data_to_fill
            .iter()
            .flat_map(|x| x["golfers"].as_array().unwrap())
            .collect::<Vec<_>>();
        // check golfer contains name and espn_id
        for golfer in golfers_check {
            if !golfer.is_object() {
                return Err(
                    "The json key golfers is not in the correct format. Expected objects."
                        .to_string(),
                );
            }
            if !golfer["name"].is_string() || !golfer["espn_id"].is_number() {
                return Err(
                    "The json key golfers is not in the correct format. Expected objects with keys name and espn_id.".to_string()
                );
            }
        }
        let event_user_player_check = data_to_fill
            .iter()
            .flat_map(|x| x["event_user_player"].as_array().unwrap())
            .collect::<Vec<_>>();
        // check event_user_player contains bettor and golfer_espn_id
        for event_user_player in event_user_player_check {
            if !event_user_player.is_object() {
                return Err(
                    "The json key event_user_player is not in the correct format. Expected objects.".to_string()
                );
            }
            if !event_user_player["bettor"].is_string()
                || !event_user_player["golfer_espn_id"].is_number()
            {
                return Err(
                    "The json key event_user_player is not in the correct format. Expected objects with keys bettor and golfer_espn_id.".to_string()
                );
            }
        }
    }

    Ok(())
}
