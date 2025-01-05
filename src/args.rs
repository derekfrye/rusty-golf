use clap::Parser;
// use serde_json::Value;
use sqlx_middleware::db::DatabaseType;
// use std::fs::{self, File, OpenOptions};
// use std::io::{Read, Write};
// use std::path::PathBuf;
// use clap::builder::ValueParser;

pub fn args_checks() -> Args {
    let xx = Args::parse();
    xx
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Database type: sqlite or postgres
    #[arg(
        short = 'd',
        long,
        value_name = "Database",
        default_value = "Sqlite",
        value_parser = clap::value_parser!(DatabaseType))
    ]
    pub db: DatabaseType,
}

// impl Args {
//     /// Validate the secrets based on the mode
//     pub fn validate(&self) -> Result<(), String> {
//         if let Mode::SecretRefresh = self.mode {
//             if let Some(client_id) = &self.secrets_client_id {
//                 if client_id.len() != 8 {
//                     return Err(
//                         "secrets_client_id must be exactly 8 characters long when Mode is Secrets."
//                             .to_string(),
//                     );
//                 }
//             }

//             if let Some(client_secret) = &self.secrets_client_secret_path {
//                 if let Err(e) = check_readable_file(client_secret.to_str().unwrap()) {
//                     return Err(e);
//                 }
//             }
//         } else if let Mode::SecretRetrieve = self.mode {
//             if let Some(client_id) = &self.secrets_client_id {
//                 if client_id.len() != 36 {
//                     return Err("Azure client_id must be 36 characters long.".to_string());
//                 }
//             }

//             if let Some(client_secret) = &self.secrets_client_secret_path {
//                 if let Err(e) = check_readable_file(client_secret.to_str().unwrap()) {
//                     return Err(e);
//                 }
//             }

//             if let Some(output_json) = &self.secret_mode_output_json {
//                 if let Err(e) = check_parent_dir_is_writeable(output_json.to_str().unwrap()) {
//                     return Err(e);
//                 }
//             }

//             if let Some(input_json) = &self.secret_mode_input_json {
//                 if let Err(e) = check_valid_json_file(input_json.to_str().unwrap()) {
//                     return Err(e);
//                 }
//             }
//         }
//         Ok(())
//     }
// }

// fn check_valid_db_type(value: &str) -> Result<(), String> {
//     match value {
//         "sqlite" => Ok(()),
//         "postgres" => Ok(()),
//         _ => Err("Database type must be either sqlite or postgres".to_string()),
//     }
// }
