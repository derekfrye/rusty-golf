use clap::Parser;
use serde_json::Value;
use sql_middleware::middleware::DatabaseType;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Database type: sqlite or postgres
    #[arg(
        short = 'd',
        long,
        value_name = "DATABASE_TYPE",
        default_value = "Sqlite",
        value_parser = clap::value_parser!(DatabaseType)
    )]
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
    #[arg(long, value_name = "DATABASE_STARTUP_SCRIPT", value_parser = crate::args::validation::check_readable_file)]
    pub db_startup_script: Option<String>,
    #[arg(
        long,
        value_name = "DATABASE_STARTUP_SCRIPT",
        value_parser = crate::args::validation::check_readable_file_and_json
    )]
    pub db_populate_json: Option<Value>,
}

#[derive(Debug, Clone)]
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
