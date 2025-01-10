use clap::Parser;
use sqlx_middleware::db::DatabaseType;

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
        value_name = "DATABASE_TYPE",
        default_value = "Sqlite",
        value_parser = clap::value_parser!(DatabaseType))
    ]
    pub db_type: DatabaseType,
    #[arg(short = 'h', long, value_name = "DATABASE_HOST", default_value = "localhost")]
    pub db_host: String,
    #[arg(short = 'p', long, value_name = "DATABASE_PORT", default_value = "5432")]
    pub db_port: Option<u16>,
    #[arg(short = 'u', long, value_name = "DATABASE_USER", default_value = "postgres")]
    pub db_user: Option<String>,
    #[arg(short = 'w', long, value_name = "DATABASE_PASSWORD")]
    pub db_password: Option<String>,
    #[arg(short = 'n', long, value_name = "DATABASE_NAME", default_value = "golf")]
    pub db_name: Option<String>,
}

impl Args {
    /// Validate the secrets based on the mode
    pub fn validate(&self) -> Result<(), String> {
        if self.db_type == DatabaseType::Postgres {

            let secrets_locations = vec!["/secrets/db_password", "/run/secrets/db_password"];

            if self.db_user.is_none() {
                return Err("Postgres user is required".to_string());
            }
            if self.db_password.is_none() {
                return Err("Postgres password is required".to_string());
            }
            if self.db_password.is_some() && secrets_locations.contains(&self.db_password.as_deref().unwrap())  {
                // open the file and read the contents
                let contents = std::fs::read_to_string("/secrets/db_password")
                    .unwrap_or("tempPasswordWillbeReplacedIn!AdminPanel".to_string());
                // set the password to the contents of the file
                db_pwd = contents.trim().to_string();
            }
        }
        Ok(())
    }
}