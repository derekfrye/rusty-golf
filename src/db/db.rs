use crate::model::{ResultStatus, Scores, Statistic};

use ::function_name::named;
use chrono::NaiveDateTime;
use deadpool_postgres::Config; //, Pool, PoolError, Runtime};
                               // use std::env;
                               // use tokio_postgres::{Error as PgError, NoTls, Row};

use crate::admin::model::admin_model::MissingDbObjects;

use sqlx::{self, sqlite::SqliteConnectOptions, Column, ConnectOptions, Pool, Row};

#[derive(Debug, Clone)]
pub enum DbPool {
    Postgres(Pool<sqlx::Postgres>),
    Sqlite(Pool<sqlx::Sqlite>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseType {
    Postgres,
    Sqlite,
}

#[derive(Clone, Debug)]
pub struct DbConfigAndPool {
    pool: DbPool,
    // db_type: DatabaseType,
}

impl DbConfigAndPool {
    pub async fn new(config: Config, db_type: DatabaseType) -> Self {
        if config.dbname.is_none() {
            panic!("dbname is required");
        }
        if db_type != DatabaseType::Sqlite {
            if config.host.is_none() {
                panic!("host is required");
            }

            if config.port.is_none() {
                panic!("port is required");
            }

            if config.user.is_none() {
                panic!("user is required");
            }

            if config.password.is_none() {
                panic!("password is required");
            }
        }

        let config_db_name = config.dbname.clone().unwrap();

        let connection_string = match db_type {
            DatabaseType::Postgres => {
                format!(
                    "postgres://{}:{}@{}:{}/{}",
                    config.user.unwrap(),
                    config.password.unwrap(),
                    config.host.unwrap(),
                    config.port.unwrap(),
                    config.dbname.unwrap()
                )
            }
            DatabaseType::Sqlite => {
                format!("sqlite://{}", config.dbname.unwrap())
            }
        };

        match db_type {
            DatabaseType::Postgres => {
                let pool_result = sqlx::postgres::PgPoolOptions::new()
                    .connect(&connection_string)
                    .await;
                match pool_result {
                    Ok(pool) => DbConfigAndPool {
                        pool: DbPool::Postgres(pool),
                        // db_type,
                    },
                    Err(e) => {
                        panic!("Failed to create Postgres pool: {}", e);
                    }
                }
            }
            DatabaseType::Sqlite => {
                #[cfg(debug_assertions)]
                {
                    dbg!(&connection_string);
                }
                let connect = SqliteConnectOptions::new()
                    .filename(&config_db_name)
                    .create_if_missing(true)
                    .connect()
                    .await;
                match connect {
                    Ok(_) => {}
                    Err(e) => {
                        let emessage =
                            format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
                        panic!("failed here 1, {}", emessage);
                    }
                }
                let pool_result = sqlx::sqlite::SqlitePoolOptions::new()
                    .connect(&connection_string)
                    .await;
                match pool_result {
                    Ok(pool) => DbConfigAndPool {
                        pool: DbPool::Sqlite(pool),
                        // db_type,
                    },
                    Err(e) => {
                        panic!("Failed to create SQLite pool: {}", e);
                    }
                }
            }
        }
    }
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

// Custom Value enum to support multiple data types
#[derive(Debug, Clone, PartialEq)]
pub enum RowValues {
    Int(i64),
    // Float(f64),
    Text(String),
    Bool(bool),
    Timestamp(NaiveDateTime),
    // Add other types as needed
}

impl RowValues {
    pub fn as_int(&self) -> Option<&i64> {
        if let RowValues::Int(value) = self {
            Some(value)
        } else {
            None
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        if let RowValues::Text(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
struct CustomDbRow {
    column_names: Vec<String>,
    rows: Vec<RowValues>,
}

impl CustomDbRow {
    pub fn get(&self, col_name: &str) -> Option<&RowValues> {
        // Find the index of the column name
        if let Some(index) = self.column_names.iter().position(|name| name == col_name) {
            // Get the corresponding row value by index
            self.rows.get(index)
        } else {
            None // Column name not found
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResultSet {
    pub results: Vec<CustomDbRow>,
}

pub struct QueryAndParams {
    pub query: String,
    pub params: Vec<RowValues>,
}

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

#[derive(Clone, Debug)]
pub struct Db {
    // pub config_and_pool: DbConfigAndPool,
    pub pool: DbPool,
}

impl Db {
    pub fn new(cnf: DbConfigAndPool) -> Result<Self, String> {
        // let cnf_clone = cnf.clone();
        Ok(Self {
            // config_and_pool: cnf,
            pool: cnf.pool,
        })
    }

    /// Check if tables or constraints are setup.
    pub async fn test_is_db_setup(
        &mut self,
        check_type: &CheckType,
    ) -> Result<Vec<DatabaseResult<String>>, Box<dyn std::error::Error>> {
        let mut dbresults = vec![];

        let query = include_str!("../admin/model/sql/schema/0x_tables_exist.sql");
        let query_and_params = QueryAndParams {
            query: query.to_string(),
            params: vec![],
        };
        let result = self.exec_general_query(vec![query_and_params], true).await;

        let missing_tables = match result {
            Ok(r) => {
                if r.db_last_exec_state == DatabaseSetupState::QueryReturnedSuccessfully {
                    r.return_result[0].results.clone()
                } else {
                    let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();
                    dbresult.db_last_exec_state = r.db_last_exec_state;
                    dbresult.error_message = r.error_message;
                    return Ok(vec![dbresult]);
                }
            }
            Err(e) => {
                let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
                let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();
                dbresult.error_message = Some(emessage);
                dbresults.push(dbresult);
                return Ok(dbresults);
            }
        };

        // may have to declare as Vec<String>
        let zz: Vec<_> = missing_tables
            .iter()
            .filter_map(|row| {
                let exists_index = row.column_names.iter().position(|col| col == "exists")?;
                let tbl_index = row.column_names.iter().position(|col| col == "tbl")?;

                // Check if the "exists" column value is `Value::Bool(true)` or `Value::Text("t")`
                match &row.rows[exists_index] {
                    RowValues::Bool(true) => match &row.rows[tbl_index] {
                        RowValues::Text(tbl_name) => Some(tbl_name.clone()),
                        _ => None,
                    },
                    RowValues::Text(value) if value == "t" => match &row.rows[tbl_index] {
                        RowValues::Text(tbl_name) => Some(tbl_name.clone()),
                        _ => None,
                    },
                    _ => None,
                }
            })
            .collect();

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

            if zz.iter().any(|x| x == table) {
                dbresult.db_last_exec_state = DatabaseSetupState::QueryReturnedSuccessfully;
            } else {
                dbresult.db_last_exec_state = DatabaseSetupState::MissingRelations;
            }

            dbresults.push(dbresult);
        }

        Ok(dbresults)
    }

    #[named]
    pub async fn create_tables(
        &mut self,
        tables: Vec<MissingDbObjects>,
        check_type: CheckType,
        ddl_for_validation: &[(&str, &str, &str, &str)],
    ) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
        let mut return_result: DatabaseResult<String> = DatabaseResult::<String>::default();
        return_result.db_object_name = function_name!().to_string();

        let entire_create_stms = if check_type == CheckType::Table {
            ddl_for_validation
                .iter()
                .filter(|x| tables.iter().any(|y| y.missing_object == x.0))
                .map(|af| af.1)
                // .into_iter()
                .collect::<Vec<&str>>()
            // .join("")
            // .flatten()
        } else {
            ddl_for_validation
                .iter()
                .filter(|x| tables.iter().any(|y| y.missing_object == x.2))
                .map(|af| af.3)
                // .collect::<Vec<&str>>()
                // .flatten()
                .collect::<Vec<&str>>()
            // .join("")
        };

        let result = self
            .exec_general_query(
                entire_create_stms
                    .iter()
                    .map(|x| QueryAndParams {
                        query: x.to_string(),
                        params: vec![],
                    })
                    .collect(),
                false,
            )
            .await;

        // let query_and_params = QueryAndParams {
        //     query: entire_create_stms,
        //     params: vec![],
        // };
        // let result = self.exec_general_query(vec![query_and_params], false).await;

        let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();

        match result {
            Ok(r) => {
                dbresult.db_last_exec_state = r.db_last_exec_state;
                dbresult.error_message = r.error_message;
                // r.return_result
            }
            Err(e) => {
                let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
                dbresult.error_message = Some(emessage);
            }
        };
        Ok(dbresult)
    }

    pub async fn get_title_from_db(
        &self,
        event_id: i32,
    ) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
        let query = "SELECT eventname FROM sp_get_event_name($1)";
        let query_and_params = QueryAndParams {
            query: query.to_string(),
            params: vec![RowValues::Int(event_id as i64)],
        };
        let result = self.exec_general_query(vec![query_and_params], true).await;

        let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();

        let missing_tables = match result {
            Ok(r) => {
                dbresult.db_last_exec_state = r.db_last_exec_state;
                dbresult.error_message = r.error_message;
                r.return_result[0].results.clone()
            }
            Err(e) => {
                let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
                let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();
                dbresult.error_message = Some(emessage);
                vec![]
            }
        };

        let zz: Vec<_> = missing_tables
            .iter()
            .filter_map(|row| {
                let exists_index = row.column_names.iter().position(|col| col == "eventname")?;

                match &row.rows[exists_index] {
                    RowValues::Text(value) => Some(value),

                    _ => None,
                }
            })
            .collect();
        if zz.len() > 0 {
            dbresult.return_result = zz[0].to_string();
        }
        Ok(dbresult)
    }

    pub async fn exec_general_query(
        &self,
        queries: Vec<QueryAndParams>,
        expect_rows: bool,
    ) -> Result<DatabaseResult<Vec<ResultSet>>, sqlx::Error> {
        let mut final_result = DatabaseResult::<Vec<ResultSet>>::default();

        if expect_rows {
            match &self.pool {
                DbPool::Postgres(pool) => {
                    let mut transaction = match pool.begin().await {
                        Ok(tx) => tx,
                        Err(e) => {
                            final_result.db_last_exec_state = DatabaseSetupState::QueryError;
                            final_result.error_message = Some(e.to_string());
                            return Ok(final_result);
                        }
                    };

                    for q in queries {
                        let mut query_item = sqlx::query(&q.query);

                        for param in q.params {
                            query_item = match param {
                                RowValues::Int(value) => query_item.bind(value),
                                RowValues::Text(value) => query_item.bind(value),
                                RowValues::Bool(value) => query_item.bind(value),
                                RowValues::Timestamp(value) => query_item.bind(value),
                            };
                        }

                        let rows_result = query_item.fetch_all(&mut *transaction).await;

                        match rows_result {
                            Ok(rows) => {
                                let mut result_set = ResultSet { results: vec![] };
                                for row in rows {
                                    let column_names = row
                                        .columns()
                                        .iter()
                                        .map(|c| c.name().to_string())
                                        .collect::<Vec<_>>();

                                    let values = row
                                        .columns()
                                        .iter()
                                        .map(|col| {
                                            let type_info = col.type_info().to_string();
                                            let value = match type_info.as_str() {
                                                "INT4" | "INT8" | "BIGINT" | "INTEGER" | "INT" => {
                                                    RowValues::Int(row.get::<i64, _>(col.name()))
                                                }
                                                "TEXT" => {
                                                    RowValues::Text(row.get::<String, _>(col.name()))
                                                }
                                                "BOOL" | "BOOLEAN" => {
                                                    RowValues::Bool(row.get::<bool, _>(col.name()))
                                                }
                                                "TIMESTAMP" => {
                                                    let timestamp: sqlx::types::chrono::NaiveDateTime =
                                                        row.get(col.name());
                                                    RowValues::Timestamp(timestamp)
                                                }
                                                _ => {
                                                    eprintln!("Unknown column type: {}", type_info);
                                                    unimplemented!("Unknown column type: {}", type_info)
                                                }
                                            };
                                            value
                                        })
                                        .collect::<Vec<_>>();

                                    let custom_row = CustomDbRow {
                                        column_names: column_names,
                                        rows: values,
                                    };
                                    result_set.results.push(custom_row);
                                }
                                final_result.return_result.push(result_set);
                            }
                            Err(e) => {
                                let _ = transaction.rollback().await;
                                final_result.db_last_exec_state = DatabaseSetupState::QueryError;
                                final_result.error_message = Some(e.to_string());
                                return Ok(final_result);
                            }
                        }
                    }
                    let _ = transaction.commit().await;
                    final_result.db_last_exec_state = DatabaseSetupState::QueryReturnedSuccessfully;
                }
                DbPool::Sqlite(pool) => {
                    let mut transaction = match pool.begin().await {
                        Ok(tx) => tx,
                        Err(e) => {
                            final_result.db_last_exec_state = DatabaseSetupState::QueryError;
                            final_result.error_message = Some(e.to_string());
                            return Ok(final_result);
                        }
                    };

                    for q in queries {
                        let mut query_item = sqlx::query(&q.query);

                        for param in q.params {
                            query_item = match param {
                                RowValues::Int(value) => query_item.bind(value),
                                RowValues::Text(value) => query_item.bind(value),
                                RowValues::Bool(value) => query_item.bind(value),
                                RowValues::Timestamp(value) => query_item.bind(value),
                            };
                        }

                        let rows_result = query_item.fetch_all(&mut *transaction).await;

                        match rows_result {
                            Ok(rows) => {
                                let mut result_set = ResultSet { results: vec![] };
                                for row in rows {
                                    let column_names = row
                                        .columns()
                                        .iter()
                                        .map(|c| c.name().to_string())
                                        .collect::<Vec<_>>();

                                    let values = row
                                        .columns()
                                        .iter()
                                        .map(|col| {
                                            let type_info = col.type_info().to_string();
                                            let value = match type_info.as_str() {
                                                "INTEGER" => {
                                                    RowValues::Int(row.get::<i64, _>(col.name()))
                                                }
                                                "TEXT" => RowValues::Text(
                                                    row.get::<String, _>(col.name()),
                                                ),
                                                "BOOLEAN" => {
                                                    RowValues::Bool(row.get::<bool, _>(col.name()))
                                                }
                                                "TIMESTAMP" => {
                                                    RowValues::Timestamp(row.get(col.name()))
                                                }
                                                _ => {
                                                    eprintln!("Unknown column type: {}", type_info);
                                                    unimplemented!(
                                                        "Unknown column type: {}",
                                                        type_info
                                                    )
                                                }
                                            };
                                            value
                                        })
                                        .collect::<Vec<_>>();

                                    let custom_row = CustomDbRow {
                                        column_names: column_names,
                                        rows: values,
                                    };
                                    result_set.results.push(custom_row);
                                }
                                final_result.return_result.push(result_set);
                            }
                            Err(e) => {
                                let _ = transaction.rollback().await;
                                final_result.db_last_exec_state = DatabaseSetupState::QueryError;
                                final_result.error_message = Some(e.to_string());
                                return Ok(final_result);
                            }
                        }
                    }
                    let _ = transaction.commit().await;
                    final_result.db_last_exec_state = DatabaseSetupState::QueryReturnedSuccessfully;
                }
            }
        } else {
            // expect_rows = false
            match &self.pool {
                DbPool::Postgres(pool) => {
                    let mut transaction = match pool.begin().await {
                        Ok(tx) => tx,
                        Err(e) => {
                            final_result.db_last_exec_state = DatabaseSetupState::QueryError;
                            final_result.error_message = Some(e.to_string());
                            return Ok(final_result);
                        }
                    };

                    for q in queries {
                        let mut query_item = sqlx::query(&q.query);

                        for param in q.params {
                            query_item = match param {
                                RowValues::Int(value) => query_item.bind(value),
                                RowValues::Text(value) => query_item.bind(value),
                                RowValues::Bool(value) => query_item.bind(value),
                                RowValues::Timestamp(value) => query_item.bind(value),
                            };
                        }

                        let exec_result = query_item.execute(&mut *transaction).await;

                        match exec_result {
                            Ok(_) => {
                                final_result
                                    .return_result
                                    .push(ResultSet { results: vec![] });
                            }
                            Err(e) => {
                                let _ = transaction.rollback().await;
                                final_result.db_last_exec_state = DatabaseSetupState::QueryError;
                                final_result.error_message = Some(e.to_string());
                                return Ok(final_result);
                            }
                        }
                    }
                    let _ = transaction.commit().await;
                    final_result.db_last_exec_state = DatabaseSetupState::QueryReturnedSuccessfully;
                }
                DbPool::Sqlite(pool) => {
                    let mut transaction = match pool.begin().await {
                        Ok(tx) => tx,
                        Err(e) => {
                            final_result.db_last_exec_state = DatabaseSetupState::QueryError;
                            final_result.error_message = Some(e.to_string());
                            return Ok(final_result);
                        }
                    };

                    for q in queries {
                        let mut query_item = sqlx::query(&q.query);

                        for param in q.params {
                            query_item = match param {
                                RowValues::Int(value) => query_item.bind(value),
                                RowValues::Text(value) => query_item.bind(value),
                                RowValues::Bool(value) => query_item.bind(value),
                                RowValues::Timestamp(value) => query_item.bind(value),
                            };
                        }

                        let exec_result = query_item.execute(&mut *transaction).await;

                        match exec_result {
                            Ok(_) => {
                                final_result
                                    .return_result
                                    .push(ResultSet { results: vec![] });
                            }
                            Err(e) => {
                                let _ = transaction.rollback().await;
                                final_result.db_last_exec_state = DatabaseSetupState::QueryError;
                                final_result.error_message = Some(e.to_string());
                                return Ok(final_result);
                            }
                        }
                    }
                    let _ = transaction.commit().await;
                    final_result.db_last_exec_state = DatabaseSetupState::QueryReturnedSuccessfully;
                }
            }
        }

        Ok(final_result)
    }
}

#[cfg(test)]
mod tests {
    use std::{env, vec};

    // use sqlx::query;
    use tokio::runtime::Runtime;

    use super::*;

    #[test]
    fn a_pg_create_and_update_tbls() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // env::var("DB_USER") = Ok("postgres".to_string());

            const TABLE_DDL: &[(&str, &str, &str, &str)] = &[
                (
                    "test",
                    "CREATE TABLE IF NOT EXISTS -- drop table event cascade
                    test (
                    event_id BIGSERIAL NOT NULL PRIMARY KEY,
                    espn_id BIGINT NOT NULL,
                    name TEXT NOT NULL,
                    ins_ts TIMESTAMP NOT NULL DEFAULT now()
                    );",
                    "",
                    "",
                ),
                (
                    "test_2",
                    "CREATE TABLE IF NOT EXISTS -- drop table event cascade
                    test_2 (
                    event_id BIGSERIAL NOT NULL PRIMARY KEY,
                    espn_id BIGINT NOT NULL,
                    name TEXT NOT NULL,
                    ins_ts TIMESTAMP NOT NULL DEFAULT now()
                    );",
                    "",
                    "",
                ),
            ];

            dotenv::dotenv().unwrap();

            let mut db_pwd = env::var("DB_PASSWORD").unwrap();
            if db_pwd == "/secrets/db_password" {
                // open the file and read the contents
                let contents = std::fs::read_to_string("/secrets/db_password")
                    .unwrap_or("tempPasswordWillbeReplacedIn!AdminPanel".to_string());
                // set the password to the contents of the file
                db_pwd = contents.trim().to_string();
            }
            let mut cfg = deadpool_postgres::Config::new();
            cfg.dbname = Some(env::var("DB_NAME").unwrap());
            cfg.host = Some(env::var("DB_HOST").unwrap());
            cfg.port = Some(env::var("DB_PORT").unwrap().parse::<u16>().unwrap());
            cfg.user = Some(env::var("DB_USER").unwrap());
            cfg.password = Some(db_pwd);

            let postgres_dbconfig: DbConfigAndPool =
                DbConfigAndPool::new(cfg, DatabaseType::Postgres).await;
            let mut postgres_db = Db::new(postgres_dbconfig).unwrap();

            let pg_objs = vec![
                MissingDbObjects {
                    missing_object: "test".to_string(),
                },
                MissingDbObjects {
                    missing_object: "test_2".to_string(),
                },
            ];

            // create two test tables
            let pg_create_result = postgres_db
                .create_tables(pg_objs, CheckType::Table, TABLE_DDL)
                .await
                .unwrap();

            if pg_create_result.db_last_exec_state == DatabaseSetupState::QueryError {
                eprintln!("Error: {}", pg_create_result.error_message.unwrap());
            }

            assert_eq!(
                pg_create_result.db_last_exec_state,
                DatabaseSetupState::QueryReturnedSuccessfully
            );
            assert_eq!(pg_create_result.return_result, String::default());

            const TABLE_DDL_SYNTAX_ERR: &[(&str, &str, &str, &str)] = &[(
                "testa",
                "CREATE TABLEXXXXXXXX IF NOT EXISTS -- drop table event cascade
                    test (
                    event_id BIGSERIAL NOT NULL PRIMARY KEY,
                    espn_id BIGINT NOT NULL,
                    name TEXT NOT NULL,
                    ins_ts TIMESTAMP NOT NULL DEFAULT now()
                    );",
                "",
                "",
            )];

            let pg_objs = vec![MissingDbObjects {
                missing_object: "testa".to_string(),
            }];

            // create a test table
            let pg_create_result = postgres_db
                .create_tables(pg_objs, CheckType::Table, TABLE_DDL_SYNTAX_ERR)
                .await
                .unwrap();

            assert_eq!(
                pg_create_result.db_last_exec_state,
                DatabaseSetupState::QueryError,
            );
            assert_eq!(pg_create_result.return_result, String::default());

            let query = "DELETE FROM test;";
            let params: Vec<RowValues> = vec![];
            let _ = postgres_db
                .exec_general_query(
                    vec![QueryAndParams {
                        query: query.to_string(),
                        params: params,
                    }],
                    false,
                )
                .await
                .unwrap();

            let query = "INSERT INTO test (espn_id, name, ins_ts) VALUES ($1, $2, $3)";
            let params: Vec<RowValues> = vec![
                RowValues::Int(123456),
                RowValues::Text("test name".to_string()),
                RowValues::Timestamp(
                    NaiveDateTime::parse_from_str("2021-08-06 16:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap(),
                ),
            ];
            // params.push("test name".to_string());
            let x = postgres_db
                .exec_general_query(
                    vec![QueryAndParams {
                        query: query.to_string(),
                        params: params,
                    }],
                    false,
                )
                .await
                .unwrap();

            if x.db_last_exec_state == DatabaseSetupState::QueryError {
                eprintln!("Error: {}", x.error_message.unwrap());
            }

            assert_eq!(
                x.db_last_exec_state,
                DatabaseSetupState::QueryReturnedSuccessfully
            );

            let query = "SELECT * FROM test";
            let params: Vec<RowValues> = vec![];
            let result = postgres_db
                .exec_general_query(
                    vec![QueryAndParams {
                        query: query.to_string(),
                        params: params,
                    }],
                    true,
                )
                .await
                .unwrap();

            let expected_result = DatabaseResult {
                db_last_exec_state: DatabaseSetupState::QueryReturnedSuccessfully,
                return_result: vec![CustomDbRow {
                    column_names: vec![
                        "event_id".to_string(),
                        "espn_id".to_string(),
                        "name".to_string(),
                        "ins_ts".to_string(),
                    ],
                    rows: vec![
                        RowValues::Int(1),
                        RowValues::Int(123456),
                        RowValues::Text("test name".to_string()),
                        RowValues::Timestamp(
                            NaiveDateTime::parse_from_str(
                                "2021-08-06 16:00:00",
                                "%Y-%m-%d %H:%M:%S",
                            )
                            .unwrap(),
                        ),
                    ],
                }],
                error_message: None,
                db_object_name: "exec_general_query".to_string(),
            };

            assert_eq!(
                result.db_last_exec_state,
                expected_result.db_last_exec_state
            );
            assert_eq!(result.error_message, expected_result.error_message);

            let cols_to_actually_check = vec!["espn_id", "name", "ins_ts"];

            for (index, row) in result.return_result.iter().enumerate() {
                let left: Vec<RowValues> = row.results[0]
                    .column_names
                    .iter()
                    .zip(&row.results[0].rows) // Pair column names with corresponding row values
                    .filter(|(col_name, _)| cols_to_actually_check.contains(&col_name.as_str()))
                    .map(|(_, value)| value.clone()) // Collect the filtered row values
                    .collect();

                // Get column names and row values from the expected result
                let right: Vec<RowValues> = expected_result.return_result[index]
                    .column_names
                    .iter()
                    .zip(&expected_result.return_result[index].rows) // Pair column names with corresponding row values
                    .filter(|(col_name, _)| cols_to_actually_check.contains(&col_name.as_str()))
                    .map(|(_, value)| value.clone()) // Collect the filtered row values
                    .collect();

                assert_eq!(left, right);
            }

            let query = "DROP TABLE test;";
            let params: Vec<RowValues> = vec![];
            let result = postgres_db
                .exec_general_query(
                    vec![QueryAndParams {
                        query: query.to_string(),
                        params: params,
                    }],
                    false,
                )
                .await
                .unwrap();

            assert_eq!(
                result.db_last_exec_state,
                DatabaseSetupState::QueryReturnedSuccessfully
            );

            let query = "DROP TABLE test_2;";
            let params: Vec<RowValues> = vec![];
            let result = postgres_db
                .exec_general_query(
                    vec![QueryAndParams {
                        query: query.to_string(),
                        params: params,
                    }],
                    false,
                )
                .await
                .unwrap();

            assert_eq!(
                result.db_last_exec_state,
                DatabaseSetupState::QueryReturnedSuccessfully
            );
        });
    }
}
