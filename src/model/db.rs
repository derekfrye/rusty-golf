use crate::model::model::{ResultStatus, Scores, Statistic};
use std::env;
use tokio::time::{timeout, Duration};
use tokio_postgres::{tls::NoTlsStream, Config, NoTls, Row, Socket};

pub const TABLE_NAMES: &[&str] = &[
    "event",
    "golfstatistic",
    "player",
    "golfuser",
    "event_user_player",
    "eup_statistic",
];

struct ConnectionParams {
    db_user: String,
    db_password: String,
    db_host: String,
    db_name: String,
    db_port: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DatabaseSetupState {
    NoConnection,
    MissingRelations,
    QueryReturnedSuccessfully,
    QueryError,
}

#[derive(Debug, Clone)]
pub struct DatabaseResult<T> {
    pub db_last_exec_state: DatabaseSetupState,
    pub return_result: T,
    pub error_message: Option<String>,
    pub table_or_function_name: String,
}

impl<T> DatabaseResult<T> {
    pub fn default() -> DatabaseResult<String> {
        DatabaseResult {
            db_last_exec_state: DatabaseSetupState::NoConnection,
            return_result: "".to_string(),
            error_message: None,
            table_or_function_name: "".to_string(),
        }
    }
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

pub async fn test_is_db_setup() -> Result<Vec<DatabaseResult<String>>, Box<dyn std::error::Error>> {
    let mut dbresults = vec![];

    for table in TABLE_NAMES {
        let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();
        dbresult.table_or_function_name = table.to_string();
        let state = check_table_exists(table).await;
        match state {
            Err(e) => {
                let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
                dbresult.error_message = Some(emessage);
            }
            Ok(s) => {
                dbresult.return_result = s.return_result;
                dbresult.db_last_exec_state = s.db_last_exec_state;
                dbresult.error_message = s.error_message;
            }
        }
        dbresults.push(dbresult);
    }

    Ok(dbresults)
}

async fn check_table_exists(
    table_name: &str,
) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
    let query = format!("SELECT 1 FROM {};", table_name);
    let query_params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[];
    let result = general_query_structure(&query, query_params).await;
    let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();
    dbresult.table_or_function_name = table_name.to_string();

    match result {
        Ok(r) => {
            dbresult.error_message = r.error_message;
            dbresult.db_last_exec_state = r.db_last_exec_state;
            match r.db_last_exec_state {
                DatabaseSetupState::QueryReturnedSuccessfully => {
                    dbresult.return_result = "Table exists".to_string();
                }
                DatabaseSetupState::QueryError => {
                    dbresult.return_result = "Table does not exist".to_string();
                    dbresult.db_last_exec_state = DatabaseSetupState::MissingRelations;
                }
                DatabaseSetupState::NoConnection => {
                    dbresult.return_result = "Can't connect to db".to_string();
                }
                _ => {
                    dbresult.return_result = "Table does not exist".to_string();
                }
            }
        }
        Err(e) => {
            let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
            dbresult.error_message = Some(emessage);
            dbresult.db_last_exec_state = DatabaseSetupState::NoConnection;
        }
    }
    Ok(dbresult)
}

pub async fn get_title_from_db(
    event_id: i32,
) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
    let query = "SELECT eventname FROM sp_get_event_name($1)";
    let query_params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[&event_id];
    let result = general_query_structure(query, query_params).await;
    let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();

    match result {
        Ok(r) => {
            if r.db_last_exec_state == DatabaseSetupState::QueryReturnedSuccessfully {
                dbresult.return_result = r.return_result[0].get(0);
            }
        }
        Err(e) => {
            let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
            dbresult.error_message = Some(emessage);
        }
    }

    Ok(dbresult)
}

pub async fn get_golfers_from_db(
    event_id: i32,
) -> Result<DatabaseResult<Vec<Scores>>, Box<dyn std::error::Error>> {
    let query =
        "SELECT grp, golfername, playername, eup_id, espn_id FROM sp_get_player_names($1) ORDER BY grp, eup_id";
    let query_params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[&event_id];
    let result = general_query_structure(query, query_params).await;
    let mut dbresult: DatabaseResult<Vec<Scores>> = DatabaseResult {
        db_last_exec_state: DatabaseSetupState::QueryReturnedSuccessfully,
        return_result: vec![],
        error_message: None,
        table_or_function_name: "sp_get_player_names".to_string(),
    };

    match result {
        Ok(r) => {
            if r.db_last_exec_state == DatabaseSetupState::QueryReturnedSuccessfully {
                let rows = r.return_result;
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
                dbresult.return_result = players;
            }
        }
        Err(e) => {
            let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
            dbresult.error_message = Some(emessage);
        }
    }

    Ok(dbresult)
}

async fn general_query_structure(
    query: &str,
    query_params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
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
                Ok(row) => Ok(DatabaseResult {
                    db_last_exec_state: DatabaseSetupState::QueryReturnedSuccessfully,
                    return_result: row,
                    error_message: None,
                    table_or_function_name: query.to_string(),
                }),
                Err(e) => {
                    let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
                    Ok(DatabaseResult {
                        db_last_exec_state: DatabaseSetupState::QueryError,
                        return_result: vec![],
                        error_message: Some(emessage),
                        table_or_function_name: query.to_string(),
                    })
                }
            }
        }
        Err(e) => {
            let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
            Ok(DatabaseResult {
                db_last_exec_state: DatabaseSetupState::NoConnection,
                return_result: vec![],
                error_message: Some(emessage),
                table_or_function_name: query.to_string(),
            })
        }
    }
}
