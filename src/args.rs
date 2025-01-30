use clap::Parser;
use serde_json::Value;
use sql_middleware::middleware::DatabaseType;
// use sqlx_middleware::db::DatabaseType;
use std::{fs, path::PathBuf, vec};

pub fn args_checks() -> CleanArgs {
    let mut xx = Args::parse();
    xx.validate().unwrap();
    CleanArgs::new(xx)
    // pub files: Option<Files>,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Database type: sqlite or postgres
    #[arg(
        short = 'd',
        long,
        value_name = "DATABASE_TYPE",
        default_value = "Sqlite",
        value_parser = clap::value_parser!(DatabaseType))
    ]
    pub db_type: DatabaseType,
    // Only necessary for postgres.
    #[arg(long, value_name = "DATABASE_HOST", default_value = "localhost")]
    pub db_host: Option<String>,
    #[arg(
        short = 'p',
        long,
        value_name = "DATABASE_PORT",
        default_value = "5432"
    )]
    pub db_port: Option<u16>,
    #[arg(
        short = 'u',
        long,
        value_name = "DATABASE_USER",
        default_value = "postgres"
    )]
    pub db_user: Option<String>,
    #[arg(short = 'w', long, value_name = "DATABASE_PASSWORD")]
    pub db_password: Option<String>,

    /// For postgres, the name of the database. For sqlite, the filename.
    #[arg(short = 'n', long, value_name = "DATABASE_NAME")]
    pub db_name: String,
    /// If specified, this sql is run on program startup. Be careful with the SQL you run here, don't mess up your own database.
    #[arg(
        long,
        value_name = "DATABASE_STARTUP_SCRIPT",
        value_parser = check_readable_file
    )]
    pub db_startup_script: Option<String>,
    #[arg(
        long,
        value_name = "DATABASE_STARTUP_SCRIPT",
        value_parser = check_readable_file_and_json
    )]
    pub db_populate_json: Option<Value>,
}

pub struct CleanArgs {
    pub db_type: DatabaseType,
    pub db_host: Option<String>,
    pub db_port: Option<u16>,
    pub db_user: Option<String>,
    pub db_password: Option<String>,
    pub db_name: String,
    pub db_startup_script: Option<String>,
    pub db_populate_json: Option<Value>,
    pub combined_sql_script: String,
}

impl Args {
    /// Validate the secrets based on the mode
    pub fn validate(&mut self) -> Result<(), String> {
        if self.db_type == DatabaseType::Postgres {
            let secrets_locations = vec!["/secrets/db_password", "/run/secrets/db_password"];

            if self.db_user.is_none() {
                return Err("Postgres user is required".to_string());
            }
            if self.db_host.is_none() || self.db_host.as_deref().unwrap().is_empty() {
                return Err("Postgres host is required".to_string());
            }
            if self.db_port.is_none() {
                return Err("Postgres port is required".to_string());
            }
            if self.db_password.is_none() {
                return Err("Postgres password is required".to_string());
            } else if secrets_locations.contains(&self.db_password.as_deref().unwrap()) {
                // open the file and read the contents
                let contents = std::fs::read_to_string("/secrets/db_password")
                    .unwrap_or("tempPasswordWillbeReplacedIn!AdminPanel".to_string());
                // set the password to the contents of the file
                self.db_password = Some(contents.trim().to_string());
            }
        }
        Ok(())
    }
}

impl CleanArgs {
    pub fn new(args: Args) -> Self {
        let mut combined_sql_script = args.db_startup_script.clone().unwrap_or_default();
        if let Some(db_startup_script) = &args.db_startup_script {
            let files = db_startup_script.split(';');
            let mut full_script = String::new();
            for file in files {
                let script = fs::read_to_string(file).unwrap();
                full_script.push_str(&script);
                // push a newline just in case
                full_script.push_str("\n");
            }
            combined_sql_script = full_script;
        }
        CleanArgs {
            db_type: args.db_type,
            db_host: args.db_host,
            db_port: args.db_port,
            db_user: args.db_user,
            db_password: args.db_password,
            db_name: args.db_name,
            db_startup_script: args.db_startup_script,
            combined_sql_script,
            db_populate_json: args.db_populate_json,
        }
    }
}

fn check_readable_file(file: &str) -> Result<String, String> {
    // split by semi-colon
    let files = file.split(';');
    let mut results = vec![];
    for file in files {
        let path = PathBuf::from(file);
        // print cwd
        // if let Ok(current_dir) = std::env::current_dir() {
        //     dbg!(current_dir);
        // } else {
        //     eprintln!("Failed to get current directory");
        // }
        if !path.is_file() || !fs::metadata(&path).is_ok() {
            return Err(format!(
                "The sql startup script '{}' is not readable.",
                file
            ));
        } else {
            results.push(path.to_str().unwrap().to_string());
        }
    }
    Ok(file.to_string())
}

fn check_readable_file_and_json(file: &str) -> Result<Value, String> {
    let path = PathBuf::from(file);
    if !path.is_file() || !fs::metadata(&path).is_ok() {
        return Err(format!("The json file '{}' is not readable.", file));
    }
    let contents = fs::read_to_string(&path).unwrap();
    let json: Value = serde_json::from_str(&contents).unwrap();
    validate_json_format(&json)?;
    Ok(json)
}

/// Validate the json file format
/// format we expect is this:
/// [{ "event": <int>, "year": <int>, "name":"value", "data_to_fill_if_event_and_year_missing": [
/// { "bettors": [{"PlayerName", "PlayerName2", "PlayerName3"...}]
/// , "golfers": [{"name": "Firstname Lastname", "espn_id": <int>}, {"name": "Firstname Lastname", "espn_id": <int>}, ...]
/// , "event_user_player": [{"bettor": "PlayerName", "golfer_espn_id": <int>}, {"bettor": "PlayerName", "golfer_espn_id": <int>}, ...]
/// }]}]
fn validate_json_format(json: &Value) -> Result<(), String> {
    if !json.is_object() {
        return Err("The json file is not in the correct format.".to_string());
    }

    // format we expect is this:
    // [{ "event": <int>, "year": <int>, "name": "", "data_to_fill_if_event_and_year_missing": [
    //{ "bettors": [{"PlayerName", "PlayerName2", "PlayerName3"...}]
    // , "golfers": [{"name": "Firstname Lastname", "espn_id": <int>}, {"name": "Firstname Lastname", "espn_id": <int>}, ...]
    // , "event_user_player": [{"bettor": "PlayerName", "golfer_espn_id": <int>}, {"bettor": "PlayerName", "golfer_espn_id": <int>}, ...]
    // }]}]

    // check the json against this format
    let expected_keys = vec![
        "event",
        "year",
        "name",
        "data_to_fill_if_event_and_year_missing",
    ];
    for (key, _) in json.as_object().unwrap() {
        if !expected_keys.contains(&key.as_str()) {
            return Err(format!(
                "The json file is not in the correct format. Expected keys: {:?}",
                expected_keys
            ));
        }
        let event = &json["event"];
        if !event.is_number() {
            return Err(
                "The json key event is not in the correct format. Expected a number.".to_string(),
            );
        }
        let year = &json["year"];
        if !year.is_number() {
            return Err(
                "The json key year is not in the correct format. Expected a number.".to_string(),
            );
        }
        let name = &json["name"];
        if !name.is_string() {
            return Err(
                "The json key name is not in the correct format. Expected a string.".to_string(),
            );
        }
    }

    // now check the data_to_fill_if_event_and_year_missing
    let data_to_fill = json["data_to_fill_if_event_and_year_missing"]
        .as_array()
        .unwrap();
    for data in data_to_fill {
        let expected_keys = vec!["bettors", "golfers", "event_user_player"];
        for (key, _) in data.as_object().unwrap() {
            if !expected_keys.contains(&key.as_str()) {
                return Err(format!("The json key data_to_fill_if_event_and_year_missing is not in the correct format. Expected keys: {:?}", expected_keys));
            }
        }
    }

    let bettors_check = data_to_fill
        .iter()
        .map(|x| x["bettors"].as_array().unwrap())
        .flatten()
        .collect::<Vec<_>>();
    // check bettors are just strings
    for bettor in bettors_check {
        if !bettor.is_string() {
            return Err(
                "The json key bettors is not in the correct format. Expected strings.".to_string(),
            );
        }
    }

    let golfers_check = data_to_fill
        .iter()
        .map(|x| x["golfers"].as_array().unwrap())
        .flatten()
        .collect::<Vec<_>>();
    // check golfer contains name and espn_id
    for golfer in golfers_check {
        if !golfer.is_object() {
            return Err(
                "The json key golfers is not in the correct format. Expected objects.".to_string(),
            );
        }
        if !golfer["name"].is_string() || !golfer["espn_id"].is_number() {
            return Err("The json key golfers is not in the correct format. Expected objects with keys name and espn_id.".to_string());
        }
    }
    let event_user_player_check = data_to_fill
        .iter()
        .map(|x| x["event_user_player"].as_array().unwrap())
        .flatten()
        .collect::<Vec<_>>();
    // check event_user_player contains bettor and golfer_espn_id
    for event_user_player in event_user_player_check {
        if !event_user_player.is_object() {
            return Err(
                "The json key event_user_player is not in the correct format. Expected objects."
                    .to_string(),
            );
        }
        if !event_user_player["bettor"].is_string()
            || !event_user_player["golfer_espn_id"].is_number()
        {
            return Err("The json key event_user_player is not in the correct format. Expected objects with keys bettor and golfer_espn_id.".to_string());
        }
    }

    Ok(())
}
