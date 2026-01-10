use super::types::Args;
use sql_middleware::middleware::DatabaseType;

impl Args {
    /// Validate the secrets based on the mode
    ///
    /// # Errors
    ///
    /// Will return `Err` if the database configuration is invalid
    ///
    /// # Panics
    ///
    /// Will panic if the password file is not found
    pub fn validate(&mut self) -> Result<(), String> {
        if self.db_type == DatabaseType::Postgres {
            let secrets_locations = ["/secrets/db_password", "/run/secrets/db_password"];

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
