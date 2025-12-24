use clap::Parser;
use std::fs;

pub mod database;
pub mod types;
pub mod validation;

pub use types::{Args, CleanArgs};

/// # Panics
///
/// Will panic if the arguments are invalid
#[must_use]
pub fn args_checks() -> CleanArgs {
    let mut xx = Args::parse();
    xx.validate().unwrap();
    CleanArgs::new(xx)
}

impl CleanArgs {
    #[must_use]
    pub fn new(args: Args) -> Self {
        let mut combined_sql_script = args.db_startup_script.clone().unwrap_or_default();
        if let Some(db_startup_script) = &args.db_startup_script {
            let files = db_startup_script.split(';');
            let mut full_script = String::new();
            for file in files {
                let file = file.trim();
                if file.is_empty() {
                    continue;
                }

                match fs::read_to_string(file) {
                    Ok(script) => {
                        full_script.push_str(&script);
                        // push a newline just in case
                        full_script.push('\n');
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to read SQL startup script '{file}': {e}");
                        // Continue with other files rather than failing completely
                    }
                }
            }
            if !full_script.is_empty() {
                combined_sql_script = full_script;
            }
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
