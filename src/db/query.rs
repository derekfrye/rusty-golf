use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod, Runtime};
use std::env;
use tokio_postgres::{NoTls, Row};

use crate::db::db::{DatabaseResult, DatabaseSetupState};

#[derive(Debug)]
pub struct Query {
    config: Config,
    pool: Option<Pool>,
}

impl Query {
    /// 1. Initializes the `Query` object with a default `Config`.
    pub fn new() -> Self {
        let config = Config::new();
        Query { config, pool: None }
    }

    /// 2. Returns a mutable reference to `Config` so the user can modify it.
    pub fn get_config(&mut self) -> &mut Config {
        &mut self.config
    }

    /// 3. Creates and stores a `Pool` if not already created, then returns it.
    pub fn get_pool(&mut self) -> Result<&Pool, Box<dyn std::error::Error>> {
        if self.pool.is_none() {
            // Set default values or use environment variables
            self.config.dbname = Some(env::var("DB_NAME").unwrap());
            self.config.host = Some(env::var("DB_HOST").unwrap());
            self.config.port = Some(env::var("DB_PORT").unwrap().parse::<u16>().unwrap());
            self.config.user = Some(env::var("DB_USER").unwrap());
            // Handle `DB_PASSWORD` and secret file reading
            let mut db_pwd = env::var("DB_PASSWORD").unwrap_or_else(|_| "password".to_string());
            if db_pwd == "/secrets/db_password" {
                let contents = match std::fs::read_to_string("/secrets/db_password") {
                    Ok(content) => content,
                    Err(e) => {
                        return Err(Box::new(e));
                    }
                };
                db_pwd = contents.trim().to_string();
            }
            self.config.password = Some(db_pwd);

            self.config.manager = Some(ManagerConfig {
                recycling_method: RecyclingMethod::Fast,
            });

            let pool = match self.config.create_pool(Some(Runtime::Tokio1), NoTls) {
                Ok(p) => p,
                Err(e) => {
                    return Err(Box::new(e));
                }
            };
            self.pool = Some(pool);
        }

        // Safely return a reference to the pool
        match self.pool.as_ref() {
            Some(pool) => Ok(pool),
            None => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Pool is not initialized",
            ))),
        }
    }

    /// 4. Executes a general query, initializing the `Pool` if necessary.
    pub async fn exec_general_query(
        &mut self,
        query: &str,
        query_params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<DatabaseResult<Vec<Row>>, Box<dyn std::error::Error>> {
        // Get the pool, handling errors explicitly
        let pool = match self.get_pool() {
            Ok(p) => p,
            Err(e) => {
                return Ok(self.create_error_result(e));
            }
        };

        // Get a client from the pool
        let client = match pool.get().await {
            Ok(c) => c,
            Err(e) => {
                return Ok(self.create_error_result(e));
            }
        };

        // Prepare the statement
        let stmt = match client.prepare(query).await {
            Ok(s) => s,
            Err(e) => {
                return Ok(self.create_error_result(e));
            }
        };

        // Execute the query
        let rows = match client.query(&stmt, query_params).await {
            Ok(r) => r,
            Err(e) => {
                return Ok(self.create_error_result(e));
            }
        };

        // Return the result wrapped in `DatabaseResult`
        Ok(DatabaseResult {
            db_last_exec_state: DatabaseSetupState::QueryReturnedSuccessfully,
            return_result: rows,
            error_message: None,
            db_object_name: "".to_string(),
        })
    }
    fn create_error_result<E>(&self, e: E) -> DatabaseResult<Vec<Row>>
    where
        E: std::fmt::Display,
    {
        let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
        let mut result = DatabaseResult::<Vec<Row>>::default();
        result.db_last_exec_state = DatabaseSetupState::NoConnection;
        result.error_message = Some(emessage);
        result
    }
}
