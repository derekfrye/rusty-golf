use crate::model::model::{ResultStatus, Scores, Statistic};
use std::env;
use tokio::time::{timeout, Duration};
use tokio_postgres::{tls::NoTlsStream, Config, NoTls, Socket};

struct ConnectionParams {
    db_user: String,
    db_password: String,
    db_host: String,
    db_name: String,
    db_port: String,
}

#[derive(Debug, Clone)]
pub struct DatabaseResult<T> {
    pub state: DatabaseSetupState,
    pub message: T,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DatabaseSetupState {
    NoConnection,
    NoRelations,
    StandardResult,
}

impl ConnectionParams {
    pub fn new() -> Self {
        ConnectionParams {
            db_user: env::var("DB_USER").unwrap(),
            db_password: env::var("DB_PASSWORD").unwrap(),
            db_host: env::var("DB_HOST").unwrap(),
            db_name: env::var("DB_NAME").unwrap(),
            db_port: env::var("DB_PORT").unwrap(),
        }
    }

    fn read_password_from_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.db_password == "/secrets/db_password" {
            // open the file and read the contents
            let contents = std::fs::read_to_string("/secrets/db_password")?;
            // set the password to the contents of the file
            self.db_password = contents.trim().to_string();
        }

        Ok(())
    }

    pub async fn return_client_and_connection(
        &mut self,
    ) -> Result<
        (
            tokio_postgres::Client,
            tokio_postgres::Connection<Socket, NoTlsStream>,
        ),
        Box<dyn std::error::Error>,
    > {
        let mut config = Config::new();
        self.read_password_from_file().unwrap();
        config
            .host(&self.db_host)
            .port(self.db_port.parse::<u16>().unwrap())
            .user(&self.db_user)
            .password(self.db_password.clone())
            .dbname(&self.db_name)
            .connect_timeout(std::time::Duration::from_secs(5));

        let connection_result = timeout(Duration::from_secs(5), config.connect(NoTls)).await;

        match connection_result {
            // Connection completed within timeout
            Ok(Ok((client, conn))) => Ok((client, conn)),
            // Connection attempt returned an error within timeout
            Ok(Err(e)) => Err(Box::new(e)),
            // Connection attempt timed out
            Err(e) => Err(Box::new(e)),
        }
    }
}

pub async fn test_is_db_setup() -> Result<DatabaseSetupState, Box<dyn std::error::Error>> {
    let mut conn_params = ConnectionParams::new();
    let x = conn_params.return_client_and_connection().await;

    match x {
        Ok((client, conn)) => {
            tokio::spawn(async move {
                if let Err(e) = conn.await {
                    eprintln!("connection error: {}", e);
                }
            });

            let row = client
                .query_one("SELECT count(*) from event;", &[])
                .await
                .unwrap();
            let one: i32 = row.get(0);

            if one == 1 {
                Ok(DatabaseSetupState::NoRelations)
            } else {
                Err("Database not setup".into())
            }
        }
        Err(e) => {
            eprintln!(
                "Failed to connect in test_is_db_setup: {}, db_host: {}",
                e, conn_params.db_host
            );
            // Err(e)
            Ok(DatabaseSetupState::NoConnection)
        }
    }
}

pub async fn get_title_from_db(
    event_id: i32,
) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
    let mut conn_params = ConnectionParams::new();
    let x = conn_params.return_client_and_connection().await;

    match x {
        Ok((client, conn)) => {
            tokio::spawn(async move {
                if let Err(e) = conn.await {
                    eprintln!("connection error: {}", e);
                }
            });

            let row = client
                .query_one("SELECT eventname FROM sp_get_event_name($1)", &[&event_id])
                .await
                .unwrap();

            let title: String = row.get(0);
            Ok(DatabaseResult {
                state: DatabaseSetupState::StandardResult,
                message: title,
            })
        }
        Err(e) => {
            eprintln!(
                "Failed to connect in test_is_db_setup: {}, db_host: {}",
                e, conn_params.db_host
            );
            // Err(e)
            Ok(DatabaseResult {
                state: DatabaseSetupState::NoConnection,
                message: "Database not setup".to_string(),
            })
        }
    }
}

pub async fn get_golfers_from_db(
    event_id: i32,
) -> Result<DatabaseResult<Vec<Scores>>, Box<dyn std::error::Error>> {
    let mut conn_params = ConnectionParams::new();
    let x = conn_params.return_client_and_connection().await;

    match x {
        Ok((client, conn)) => {
            tokio::spawn(async move {
                if let Err(e) = conn.await {
                    eprintln!("connection error: {}", e);
                }
            });

            let rows = client
                .query(
                    "SELECT grp, golfername, playername, eup_id, espn_id FROM sp_get_player_names($1) ORDER BY grp, eup_id",
                    &[&event_id]
                ).await
                .unwrap();

            let players = rows
                .iter()
                .map(|row| Scores {
                    // parse column 0 as an int32
                    group: row.get::<_, i64>(0),
                    golfer_name: row.get(1),
                    bettor_name: row.get(2),
                    eup_id: row.get::<_, i64>(3),
                    espn_id: row.get::<_, i64>(4),
                    detailed_statistics: Statistic {
                        eup_id: row.get::<_, i64>(3),
                        rounds: vec![],
                        scores: vec![],
                        tee_times: vec![],
                        holes_completed: vec![],
                        success_fail: ResultStatus::NoData,
                        total_score: 0,
                    },
                })
                .collect();

            Ok(DatabaseResult {
                state: DatabaseSetupState::StandardResult,
                message: players,
            })
        }
        Err(e) => {
            eprintln!(
                "Failed to connect in test_is_db_setup: {}, db_host: {}",
                e, conn_params.db_host
            );
            // Err(e)
            Ok(DatabaseResult {
                state: DatabaseSetupState::NoConnection,
                message: vec![],
            })
        }
    }
}
