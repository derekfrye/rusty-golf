use crate::model::{ResultStatus, Scores, Statistic};
use ::function_name::named;
use std::env;
use tokio::time::{timeout, Duration};
use tokio_postgres::{tls::NoTlsStream, Config, NoTls, Row, Socket};

use crate::admin::model::admin_model::MissingTables;

pub const TABLES_AND_CREATE_SQL: &[(&str, &str, &str, &str)] = &[
    (
        "event",
        include_str!("admin/model/sql/schema/00_event.sql"),
        "",
        "",
    ),
    (
        "golfstatistic",
        include_str!("admin/model/sql/schema/01_golfstatistic.sql"),
        "",
        "",
    ),
    (
        "player",
        include_str!("admin/model/sql/schema/02_player.sql"),
        "",
        "",
    ),
    (
        "golfuser",
        include_str!("admin/model/sql/schema/03_golfuser.sql"),
        "",
        "",
    ),
    (
        "event_user_player",
        include_str!("admin/model/sql/schema/04_event_user_player.sql"),
        "",
        "",
    ),
    (
        "eup_statistic",
        include_str!("admin/model/sql/schema/05_eup_statistic.sql"),
        "",
        "",
    ),
];

pub const TABLES_AND_CONSTRAINTS: &[(&str, &str, &str, &str)] = &[
    (
        "player",
        "UNIQUE",
        "unq_name",
        include_str!("admin/model/sql/constraints/01_player.sql"),
    ),
    (
        "player",
        "UNIQUE",
        "unq_espn_id",
        include_str!("admin/model/sql/constraints/02_player.sql"),
    ),
    (
        "event_user_player",
        "UNIQUE",
        "unq_event_id_user_id_player_id",
        include_str!("admin/model/sql/constraints/03_event_user_player.sql"),
    ),
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

#[derive(Debug, Clone, PartialEq)]
pub enum CheckType {
    Table,
    Constraint,
}

#[derive(Debug, Clone)]
pub struct DatabaseResult<T: Default> {
    pub db_last_exec_state: DatabaseSetupState,
    pub return_result: T,
    pub error_message: Option<String>,
    pub table_or_function_name: String,
}

impl<T: Default> DatabaseResult<T> {
    pub fn default() -> DatabaseResult<T> {
        DatabaseResult {
            db_last_exec_state: DatabaseSetupState::NoConnection,
            return_result: Default::default(),
            error_message: None,
            table_or_function_name: "".to_string(),
        }
    }
}

impl Default for DatabaseResult<Vec<Row>> {
    fn default() -> Self {
        DatabaseResult {
            db_last_exec_state: DatabaseSetupState::NoConnection,
            return_result: vec![],
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

#[named]
/// Check if tables are setup. Does not check if constraints are setup; assumes those were created during table creation.
pub async fn test_is_db_setup() -> Result<Vec<DatabaseResult<String>>, Box<dyn std::error::Error>> {
    let mut dbresults = vec![];

    for table in TABLES_AND_CREATE_SQL.iter() {
        let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();
        dbresult.table_or_function_name = table.0.to_string();
        let state = check_table_exists(table.0, CheckType::Table, function_name!()).await;
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

#[named]
async fn create_tbl(
    ddl: &[(&str, &str, &str, &str)],
    table: String,
    check_type: CheckType,
) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
    let state = check_table_exists(&table, check_type, function_name!()).await;
    let mut return_result: DatabaseResult<String> = DatabaseResult::<String>::default();
    return_result.table_or_function_name = function_name!().to_string();

    match state {
        Err(e) => {
            let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
            return_result.db_last_exec_state = DatabaseSetupState::NoConnection;
            return_result.error_message = Some(emessage);
        }
        Ok(s) => {
            if s.db_last_exec_state == DatabaseSetupState::MissingRelations {
                let query = ddl.iter().find(|x| x.0 == table).unwrap().1;
                let result = exec_general_query(&query, &[]).await;
                match result {
                    Ok(r) => {
                        if r.db_last_exec_state != DatabaseSetupState::QueryReturnedSuccessfully {
                            let emessage = format!(
                                "Failed in {}, {}: {}",
                                std::file!(),
                                std::line!(),
                                r.error_message.clone().unwrap_or("".to_string())
                            );
                            return_result.db_last_exec_state = DatabaseSetupState::QueryError;
                            return_result.error_message = Some(emessage);
                            return_result.table_or_function_name = r.table_or_function_name;
                        } else {
                            // table created successfully
                            return_result.db_last_exec_state = r.db_last_exec_state;
                            return_result.table_or_function_name = r.table_or_function_name;
                        }
                    }
                    Err(e) => {
                        let emessage =
                            format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
                        return_result.db_last_exec_state = DatabaseSetupState::QueryError;
                        return_result.error_message = Some(emessage);
                    }
                }
            } else {
                return_result = s;
            }
        }
    }
    Ok(return_result)
}

#[named]
pub async fn create_tables(
    tables: Vec<MissingTables>,
    check_type: CheckType,
) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
    let mut return_result: DatabaseResult<String> = DatabaseResult::<String>::default();
    return_result.table_or_function_name = function_name!().to_string();

    if check_type == CheckType::Table {
        for table in tables.iter().take_while(|xa| {
            TABLES_AND_CREATE_SQL
                .iter()
                .any(|af| af.0 == xa.missing_table.as_str())
        }) {
            let create_table_attempt = create_tbl(
                TABLES_AND_CREATE_SQL,
                table.missing_table.clone(),
                CheckType::Table,
            )
            .await;
            match create_table_attempt {
                Ok(a) => {
                    if a.db_last_exec_state != DatabaseSetupState::QueryReturnedSuccessfully {
                        return_result = a;
                        return Ok(return_result);
                    }
                }
                Err(e) => {
                    let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
                    return_result.db_last_exec_state = DatabaseSetupState::QueryError;
                    return_result.error_message = Some(emessage);
                    return Ok(return_result);
                }
            }
        }

        return_result.db_last_exec_state = DatabaseSetupState::QueryReturnedSuccessfully;
    } else {
        for table in TABLES_AND_CONSTRAINTS.iter() {
            let create_constraint_attempt = create_tbl(
                TABLES_AND_CONSTRAINTS,
                table.0.to_string(),
                CheckType::Constraint,
            )
            .await;
            match create_constraint_attempt {
                Ok(a) => {
                    if a.db_last_exec_state != DatabaseSetupState::QueryReturnedSuccessfully {
                        return_result = a;
                        return Ok(return_result);
                    }
                }
                Err(e) => {
                    let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
                    return_result.db_last_exec_state = DatabaseSetupState::QueryError;
                    return_result.error_message = Some(emessage);
                    return Ok(return_result);
                }
            }
        }

        return_result.db_last_exec_state = DatabaseSetupState::QueryReturnedSuccessfully;
    }

    Ok(return_result)
}

async fn check_table_exists(
    table_name: &str,
    check_type: CheckType,
    calling_function: &str,
) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
    let query: String;
    let query_params_storage: Vec<&(dyn tokio_postgres::types::ToSql + Sync)>;
    let constraint_type: &str;
    let constraint_name: &str;

    if check_type == CheckType::Table {
        query = format!("SELECT 1 FROM {} LIMIT 1;", table_name);
        query_params_storage = vec![];
    } else {
        query = format!(
            "SELECT 1 FROM information_schema.table_constraints WHERE table_name = $1 AND constraint_type = $2 and constraint_name = $3 LIMIT 1;"
        );
        constraint_type = TABLES_AND_CONSTRAINTS
            .iter()
            .find(|x| x.0 == table_name)
            .unwrap()
            .1;
        constraint_name = TABLES_AND_CONSTRAINTS
            .iter()
            .find(|x| x.0 == table_name)
            .unwrap()
            .2;
        query_params_storage = vec![&table_name, &constraint_type, &constraint_name];
    }

    // dbg!(&query);

    let query_params = &query_params_storage[..];
    let result = exec_general_query(&query, query_params).await;

    if cfg!(debug_assertions)
        && table_name == "event"
        && check_type == CheckType::Table
        && calling_function == "create_tbl"
    {
        println!("here.");
    }

    let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();
    dbresult.table_or_function_name = table_name.to_string();

    match result {
        Ok(r) => {
            dbresult.error_message = r.error_message;
            dbresult.db_last_exec_state = r.db_last_exec_state;
            match r.db_last_exec_state {
                DatabaseSetupState::QueryReturnedSuccessfully => {
                    if check_type == CheckType::Table {
                        dbresult.return_result = "Table exists".to_string();
                    } else {
                        if r.return_result.len() > 0 && !r.return_result[0].is_empty() {
                            let xx: String = r.return_result[0].get(0);
                            if xx == "1" {
                                dbresult.return_result = "Constraint exists".to_string();
                            } else {
                                dbresult.return_result = "Constraint does not exist".to_string();
                                dbresult.db_last_exec_state = DatabaseSetupState::MissingRelations;
                            }
                        } else {
                            let _rr = r.return_result.len();
                        }
                    }
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
    let result = exec_general_query(query, query_params).await;
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
    let result = exec_general_query(query, query_params).await;
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

async fn exec_general_query(
    query: &str,
    query_params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) -> Result<DatabaseResult<Vec<Row>>, Box<dyn std::error::Error>> {
    let mut conn_params = ConnectionParams::new();
    let x = conn_params.return_client_and_connection().await;
    let mut result = DatabaseResult::<Vec<Row>>::default();
    result.table_or_function_name = query.to_string();

    match x {
        Ok((client, conn)) => {
            tokio::spawn(async move {
                if let Err(e) = conn.await {
                    eprintln!("connection error: {}", e);
                }
            });

            let row = client.query(query, query_params).await;

            if cfg!(debug_assertions)
                && query.contains("constraint_type")
                && query_params.len() >= 3
                && format!("{:?}", query_params[2]).trim_matches('"') == r#"unq_name"#
                && format!("{:?}", query_params[0]).trim_matches('"') == r#"player"#
            {
                println!("here.");
            }

            match row {
                Ok(row) => {
                    result.db_last_exec_state = DatabaseSetupState::QueryReturnedSuccessfully;
                    result.return_result = row;
                    result.error_message = None;
                }
                Err(e) => {
                    let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
                    result.db_last_exec_state = DatabaseSetupState::QueryError;
                    result.error_message = Some(emessage);
                }
            }
        }
        Err(e) => {
            let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
            result.db_last_exec_state = DatabaseSetupState::NoConnection;
            result.error_message = Some(emessage);
        }
    }

    Ok(result)
}
