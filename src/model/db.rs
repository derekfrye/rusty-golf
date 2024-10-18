use crate::model::model::{ ResultStatus, Scores, Statistic };
use std::env;
use tokio::time::{ timeout, Duration };
use tokio_postgres::{ tls::NoTlsStream, Config, NoTls, Row, Socket };

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
    MissingRelations,
    StandardResult,
    QueryError,
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
        &mut self
    ) -> Result<
        (tokio_postgres::Client, tokio_postgres::Connection<Socket, NoTlsStream>),
        Box<dyn std::error::Error>
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
    let mut result = DatabaseSetupState::NoConnection;
    let table_names = vec!["event", "golfstatistic", "player", "golfuser", "event_user_player", "eup_statistic"];

    for table in &table_names {
        let state = check_table_exists(table).await;
        match state {
            Err(e) => {
                eprintln!("Failed in {}, {}: {}", file!(), line!(), e);
                return Ok(DatabaseSetupState::NoConnection);
            }
            Ok(s) => {
                result = s;
            }
        }
    }

    Ok(result)
}

async fn check_table_exists(
    table_name: &str
) -> Result<DatabaseSetupState, Box<dyn std::error::Error>> {
    let query = format!("SELECT 1 FROM {};", table_name);
    let query_params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[];
    let result = general_query_structure(&query, query_params).await;

    match result {
        Ok(r) => {
            if r.state == DatabaseSetupState::QueryError {
                Ok(DatabaseSetupState::MissingRelations)
            } else {
                Ok(r.state)
            }
        }
        Err(e) => {
            eprintln!("Failed in {}, {}: {}", file!(), line!(), e);
            Ok(DatabaseSetupState::NoConnection)
        }
    }
}

pub async fn get_title_from_db(
    event_id: i32
) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
    let query = "SELECT eventname FROM sp_get_event_name($1)";
    let query_params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[&event_id];
    let result = general_query_structure(query, query_params).await;
    let mut dbresult = DatabaseResult {
        state: DatabaseSetupState::StandardResult,
        message: "".to_string(),
    };

    match result {
        Ok(r) => {
            if r.state == DatabaseSetupState::StandardResult {
                dbresult.message = r.message[0].get(0);
            }
        }
        Err(_) => {}
    }
    Ok(dbresult)
}

pub async fn get_golfers_from_db(
    event_id: i32
) -> Result<DatabaseResult<Vec<Scores>>, Box<dyn std::error::Error>> {
    let query =
        "SELECT grp, golfername, playername, eup_id, espn_id FROM sp_get_player_names($1) ORDER BY grp, eup_id";
    let query_params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[&event_id];
    let result = general_query_structure(query, query_params).await;
    let mut dbresult: DatabaseResult<Vec<Scores>> = DatabaseResult {
        state: DatabaseSetupState::StandardResult,
        message: vec![],
    };

    match result {
        Ok(r) => {
            if r.state == DatabaseSetupState::StandardResult {
                let rows = r.message;
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
                dbresult.message = players;
            }
        }
        Err(_) => {}
    }
    Ok(dbresult)
}

async fn general_query_structure(
    query: &str,
    query_params: &[&(dyn tokio_postgres::types::ToSql + Sync)]
) -> Result<DatabaseResult<Vec<Row>>, Box<dyn std::error::Error>> {
    let mut conn_params = ConnectionParams::new();
    let x = conn_params.return_client_and_connection().await;

    match x {
        Ok((client, conn)) => {
            tokio::spawn(async move {
                if let Err(e) = conn.await {
                    eprintln!("connection error: {}", e);
                }
            });

            let row = client.query(query, query_params).await;
            match row {
                Ok(row) => {
                    Ok(DatabaseResult {
                        state: DatabaseSetupState::StandardResult,
                        message: row,
                    })
                }
                Err(e) => {
                    eprintln!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
                    Ok(DatabaseResult {
                        state: DatabaseSetupState::QueryError,
                        message: vec![],
                    })
                }
            }
        }
        Err(e) => {
            eprintln!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
            Ok(DatabaseResult {
                state: DatabaseSetupState::NoConnection,
                message: vec![],
            })
        }
    }
}
