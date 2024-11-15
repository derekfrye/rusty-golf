use crate::model::{ResultStatus, Scores, Statistic};
use ::function_name::named;
use deadpool_postgres::{Config, ManagerConfig, RecyclingMethod, Runtime};
use std::env;
use tokio_postgres::{NoTls, Row};

use crate::admin::model::admin_model::MissingDbObjects;

pub struct Db {
    config: Config,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckType {
    Table,
    Constraint,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DatabaseSetupState {
    NoConnection,
    MissingRelations,
    QueryReturnedSuccessfully,
    QueryError,
}

pub const TABLES_AND_DDL: &[(&str, &str, &str, &str)] = &[
    (
        "event",
        include_str!("../admin/model/sql/schema/00_event.sql"),
        "",
        "",
    ),
    (
        "golfstatistic",
        include_str!("../admin/model/sql/schema/01_golfstatistic.sql"),
        "",
        "",
    ),
    (
        "player",
        include_str!("../admin/model/sql/schema/02_player.sql"),
        "",
        "",
    ),
    (
        "golfuser",
        include_str!("../admin/model/sql/schema/03_golfuser.sql"),
        "",
        "",
    ),
    (
        "event_user_player",
        include_str!("../admin/model/sql/schema/04_event_user_player.sql"),
        "",
        "",
    ),
    (
        "eup_statistic",
        include_str!("../admin/model/sql/schema/05_eup_statistic.sql"),
        "",
        "",
    ),
];

pub const TABLES_CONSTRAINT_TYPE_CONSTRAINT_NAME_AND_DDL: &[(&str, &str, &str, &str)] = &[
    (
        "player",
        "UNIQUE",
        "unq_name",
        include_str!("../admin/model/sql/constraints/01_player.sql"),
    ),
    (
        "player",
        "UNIQUE",
        "unq_espn_id",
        include_str!("../admin/model/sql/constraints/02_player.sql"),
    ),
    (
        "event_user_player",
        "UNIQUE",
        "unq_event_id_user_id_player_id",
        include_str!("../admin/model/sql/constraints/03_event_user_player.sql"),
    ),
];

#[derive(Debug, Clone)]
pub struct DatabaseResult<T: Default> {
    pub db_last_exec_state: DatabaseSetupState,
    pub return_result: T,
    pub error_message: Option<String>,
    pub db_object_name: String,
}

impl<T: Default> DatabaseResult<T> {
    pub fn default() -> DatabaseResult<T> {
        DatabaseResult {
            db_last_exec_state: DatabaseSetupState::NoConnection,
            return_result: Default::default(),
            error_message: None,
            db_object_name: "".to_string(),
        }
    }
}

impl Default for DatabaseResult<Vec<Row>> {
    fn default() -> Self {
        DatabaseResult {
            db_last_exec_state: DatabaseSetupState::NoConnection,
            return_result: vec![],
            error_message: None,
            db_object_name: "".to_string(),
        }
    }
}

impl Db {

#[named]
/// Check if tables or constraints are setup.
pub async fn test_is_db_setup(
    check_type: &CheckType,
) -> Result<Vec<DatabaseResult<String>>, Box<dyn std::error::Error>> {
    let mut dbresults = vec![];

    fn local_fn_get_iter(check_type: &CheckType) -> impl Iterator<Item = &'static str> {
        let iter = match check_type {
            CheckType::Table => TABLES_AND_DDL.iter(),
            _ => TABLES_CONSTRAINT_TYPE_CONSTRAINT_NAME_AND_DDL.iter(),
        };
        iter.map(|x| x.0)
    }

    for table in local_fn_get_iter(&check_type) {
        let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();
        dbresult.db_object_name = table.to_string();
        let state = check_obj_exists(table, &check_type, function_name!(), None).await;
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
deadpool_config: Option<&Config>,
) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
    let state = check_obj_exists(&table, &check_type, function_name!(), deadpool_config).await;
    let mut return_result: DatabaseResult<String> = DatabaseResult::<String>::default();
    return_result.db_object_name = function_name!().to_string();

    match state {
        Err(e) => {
            let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
            return_result.db_last_exec_state = DatabaseSetupState::NoConnection;
            return_result.error_message = Some(emessage);
        }
        Ok(s) => {
            if s.db_last_exec_state == DatabaseSetupState::MissingRelations {
                let query = ddl.iter().find(|x| x.0 == table).unwrap().1;
                let result = exec_general_query(&query, &[], None).await;
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
                            return_result.db_object_name = r.db_object_name;
                        } else {
                            // table created successfully
                            return_result.db_last_exec_state = r.db_last_exec_state;
                            return_result.db_object_name = r.db_object_name;
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
    tables: Vec<MissingDbObjects>,
    check_type: CheckType,
) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
    let mut return_result: DatabaseResult<String> = DatabaseResult::<String>::default();
    return_result.db_object_name = function_name!().to_string();

    if check_type == CheckType::Table {
        for table in tables.iter().take_while(|xa| {
            TABLES_AND_DDL
                .iter()
                .any(|af| af.0 == xa.missing_object.as_str())
        }) {
            let create_table_attempt = create_tbl(
                TABLES_AND_DDL,
                table.missing_object.clone(),
                CheckType::Table,
                None,
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
        for table in TABLES_CONSTRAINT_TYPE_CONSTRAINT_NAME_AND_DDL.iter() {
            let create_constraint_attempt = create_tbl(
                TABLES_CONSTRAINT_TYPE_CONSTRAINT_NAME_AND_DDL,
                table.0.to_string(),
                CheckType::Constraint,
                None,
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

async fn check_obj_exists(
    table_name: &str,
    check_type: &CheckType,
    calling_function: &str,
    deadpool_config: Option<&Config>,
) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
    let query: String;
    let query_params_storage: Vec<&(dyn tokio_postgres::types::ToSql + Sync)>;
    let constraint_type: &str;
    let constraint_name: &str;

    if check_type == &CheckType::Table {
        query = format!("SELECT 1 FROM {} LIMIT 1;", table_name);
        query_params_storage = vec![];
    } else {
        query = format!(
            "SELECT 1 FROM information_schema.table_constraints WHERE table_name = $1 AND constraint_type = $2 and constraint_name = $3 LIMIT 1;"
        );
        constraint_type = TABLES_CONSTRAINT_TYPE_CONSTRAINT_NAME_AND_DDL
            .iter()
            .find(|x| x.0 == table_name)
            .unwrap()
            .1;
        constraint_name = TABLES_CONSTRAINT_TYPE_CONSTRAINT_NAME_AND_DDL
            .iter()
            .find(|x| x.0 == table_name)
            .unwrap()
            .2;
        query_params_storage = vec![&table_name, &constraint_type, &constraint_name];
    }

    // dbg!(&query);

    let query_params = &query_params_storage[..];
    let result = exec_general_query(&query, query_params, deadpool_config).await;

    if cfg!(debug_assertions)
        && table_name == "event"
        && check_type == &CheckType::Table
        && calling_function == "create_tbl"
    {
        dbg!("here.");
    }

    let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();
    dbresult.db_object_name = table_name.to_string();

    match result {
        Ok(r) => {
            dbresult.error_message = r.error_message;
            dbresult.db_last_exec_state = r.db_last_exec_state;
            match r.db_last_exec_state {
                DatabaseSetupState::QueryReturnedSuccessfully => {
                    if check_type == &CheckType::Table {
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
    let result = exec_general_query(query, query_params, None).await;
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
    let result = exec_general_query(query, query_params, None).await;
    let mut dbresult: DatabaseResult<Vec<Scores>> = DatabaseResult {
        db_last_exec_state: DatabaseSetupState::QueryReturnedSuccessfully,
        return_result: vec![],
        error_message: None,
        db_object_name: "sp_get_player_names".to_string(),
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
    deadpool_config: Option<&Config>,
) -> Result<DatabaseResult<Vec<Row>>, Box<dyn std::error::Error>> {
    let mut cfg;
    let pool;
    if deadpool_config.is_none() {
        cfg = Config::new();

    let mut db_pwd = env::var("DB_PASSWORD").unwrap();
    if db_pwd == "/secrets/db_password" {
        // open the file and read the contents
        let contents = std::fs::read_to_string("/secrets/db_password")
            .unwrap_or("tempPasswordWillbeReplacedIn!AdminPanel".to_string());
        // set the password to the contents of the file
        db_pwd = contents.trim().to_string();
    }

    cfg.dbname = Some(env::var("DB_NAME").unwrap());
    cfg.host = Some(env::var("DB_HOST").unwrap());
    cfg.port = Some(env::var("DB_PORT").unwrap().parse::<u16>().unwrap());
    cfg.user = Some(env::var("DB_USER").unwrap());
    cfg.password = Some(db_pwd);

    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });

     pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls).unwrap();
}
else {
    cfg = deadpool_config.unwrap().clone();
    pool = cfg.get_pool_config().
}

    let pool = match cfg.create_pool(Some(Runtime::Tokio1), NoTls) {
        Ok(p) => p,
        Err(e) => return Ok(create_error_result(e)),    
    
}
    let client = match pool.get().await {
        Ok(c) => c,
        Err(e) => return Ok(create_error_result(e)),
    };

    let stmt = match client.prepare_cached(query).await {
        Ok(s) => s,
        Err(e) => return Ok(create_error_result(e)),
    };

    let row = match client.query(&stmt, query_params).await {
        Ok(r) => r,
        Err(e) => return Ok(create_error_result(e)),
    };

    if cfg!(debug_assertions)
        && query.contains("constraint_type")
        && query_params.len() >= 3
        && format!("{:?}", query_params[0]).trim_matches('"') == "player"
        && matches!(
            format!("{:?}", query_params[2]).trim_matches('"'),
            "unq_name" | "unq_espn_id"
        )
    {
        dbg!("here.");
    }

    let mut result = DatabaseResult::<Vec<Row>>::default();
    result.db_last_exec_state = DatabaseSetupState::QueryReturnedSuccessfully;
    result.return_result = row;
    result.error_message = None;
    Ok(result)
}

fn create_error_result<E>(e: E) -> DatabaseResult<Vec<Row>>
where
    E: std::fmt::Display,
{
    let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
    let mut result = DatabaseResult::<Vec<Row>>::default();
    result.db_last_exec_state = DatabaseSetupState::NoConnection;
    result.error_message = Some(emessage);
    result
}}

#[cfg(test)]
mod tests {
    use tokio::runtime::Runtime;

    use super::*;

    #[test]
    fn test_check_obj_exists_constraint() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // env::var("DB_USER") = Ok("postgres".to_string());

            const TABLE_DDL: &[(&str, &str, &str, &str)] = &[(
                "test",
                "CREATE TABLE -- drop table event cascade
                    test (
                    event_id BIGSERIAL NOT NULL PRIMARY KEY,
                    espn_id BIGINT NOT NULL,
                    name TEXT NOT NULL,
                    ins_ts TIMESTAMP NOT NULL DEFAULT now()
                    );",
                "",
                "",
            )];

            let mut deadpool_config = Config::new();
            deadpool_config.dbname = Some("deadpool".to_string());
            deadpool_config.host = Some("localhost".to_string());
            deadpool_config.port = Some(5432);
            deadpool_config.user = Some("postgres".to_string());
            deadpool_config.password = Some("postgres".to_string());

            // create a test table
            let x = create_tbl(TABLE_DDL, "test".to_string(), CheckType::Table, Some(&deadpool_config))
                .await
                .unwrap();

            assert_eq!(
                x.db_last_exec_state,
                DatabaseSetupState::QueryReturnedSuccessfully
            );
            assert_eq!(x.return_result, String::default());

            let result = check_obj_exists(
                "player",
                &CheckType::Constraint,
                "test_check_obj_exists_constraint",
                Some(&deadpool_config),
            )
            .await;
            assert!(result.is_ok());
            let db_result = result.unwrap();
            assert_eq!(db_result.db_object_name, "player");
        });
    }
}
