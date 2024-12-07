use crate::model::{ResultStatus, Scores, Statistic};

use ::function_name::named;
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

// pub enum ObjType {
//     Table,
//     Constraint,
// }

// trait PostgresParam: for<'a> sqlx::Encode<'a, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> {}
// impl<T> PostgresParam for T where
//     T: for<'a> sqlx::Encode<'a, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>
// {
// }

// trait SqliteParam: for<'a> sqlx::Encode<'a, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> {}
// impl<T> SqliteParam for T where T: for<'a> sqlx::Encode<'a, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> {}

// enum DbQueryResult {
//     Postgres(sqlx::postgres::PgQueryResult),
//     Sqlite(sqlx::sqlite::SqliteQueryResult),
// }

// enum DbRow {
//     Postgres(sqlx::postgres::PgRow),
//     Sqlite(sqlx::sqlite::SqliteRow),
// }

// enum DbQueryOne<T> {
//     Postgres(T),
//     Sqlite(T),
// }

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
enum RowValues {
    Int(i64),
    // Float(f64),
    Text(String),
    Bool(bool),
    Timestamp(String),
    // Add other types as needed
}

impl RowValues {
    // pub fn as_ref(&self) -> Option<&dyn std::fmt::Debug> {
    //     match self {
    //         RowValues::Int(value) => Some(value),
    //         // RowValues::Float(value) => Some(value),
    //         RowValues::Text(value) => Some(value),
    //         RowValues::Bool(value) => Some(value),
    //         // Extend this with other types as needed
    //     }
    // }

    pub fn as_int(&self) -> Option<&i64> {
        if let RowValues::Int(value) = self {
            Some(value)
        } else {
            None
        }
    }

    // pub fn as_float(&self) -> Option<&f64> {
    //     if let RowValues::Float(value) = self {
    //         Some(value)
    //     } else {
    //         None
    //     }
    // }

    pub fn as_text(&self) -> Option<&str> {
        if let RowValues::Text(value) = self {
            Some(value)
        } else {
            None
        }
    }

    // pub fn as_bool(&self) -> Option<&bool> {
    //     if let RowValues::Bool(value) = self {
    //         Some(value)
    //     } else {
    //         None
    //     }
    // }
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
        let params: Vec<RowValues> = vec![];
        let result = self.exec_general_query(query, params, true).await;

        let missing_tables = match result {
            Ok(r) => {
                if r.db_last_exec_state == DatabaseSetupState::QueryReturnedSuccessfully {
                    r.return_result
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
                .join("")
            // .flatten()
        } else {
            ddl_for_validation
                .iter()
                .filter(|x| tables.iter().any(|y| y.missing_object == x.2))
                .map(|af| af.3)
                // .collect::<Vec<&str>>()
                // .flatten()
                .collect::<Vec<&str>>()
                .join("")
        };

        let params: Vec<RowValues> = vec![];
        let result = self
            .exec_general_query(&entire_create_stms, params, false)
            .await;

        let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();

        match result {
            Ok(r) => {
                dbresult.db_last_exec_state = r.db_last_exec_state;
                dbresult.error_message = r.error_message;
                // r.return_result
            }
            Err(e) => {
                let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
                let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();
                dbresult.error_message = Some(emessage);
                // vec![]
            }
        };
        Ok(dbresult)
    }

    // async fn check_obj_exists(&self, obj_name: &str, typ: ObjType) -> Result<bool, sqlx::Error> {
    //     let check_sql = match &self.pool {
    //         DbPool::Postgres(_) => match typ {
    //             ObjType::Table => "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = $1);",
    //             ObjType::Constraint => "SELECT EXISTS (SELECT FROM information_schema.table_constraints WHERE table_schema = 'public' AND constraint_name = $1);",
    //         },
    //         DbPool::Sqlite(_) => match typ {
    //             ObjType::Table => "SELECT name FROM sqlite_master WHERE type='table' AND name = ?;",
    //             ObjType::Constraint => "SELECT name FROM sqlite_master WHERE type='constraint' AND name = ?;",
    //         },
    //     };
    //     let exists_result: Result<bool, sqlx::Error> = match &self.pool {
    //         DbPool::Postgres(pool) => sqlx::query_scalar::<_, bool>(check_sql)
    //             .bind(obj_name)
    //             .fetch_one(pool)
    //             .await
    //             .map(|result| result),
    //         DbPool::Sqlite(pool) => sqlx::query_scalar::<_, bool>(check_sql)
    //             .bind(obj_name)
    //             .fetch_one(pool)
    //             .await
    //             .map(|result| result),
    //     };
    //     match exists_result {
    //         Ok(exists) => {
    //             return Ok(exists);
    //         }
    //         Err(e) => {
    //             return Err(e);
    //         }
    //     }
    // }

    pub async fn get_title_from_db(
        &self,
        event_id: i32,
    ) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
        let query = "SELECT eventname FROM sp_get_event_name($1)";
        let params: Vec<RowValues> = vec![RowValues::Int(event_id as i64)];
        let result = self.exec_general_query(query, params, true).await;

        let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();

        let missing_tables = match result {
            Ok(r) => {
                dbresult.db_last_exec_state = r.db_last_exec_state;
                dbresult.error_message = r.error_message;
                r.return_result
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

    pub async fn get_golfers_from_db(
        &self,
        event_id: i32,
    ) -> Result<DatabaseResult<Vec<Scores>>, Box<dyn std::error::Error>> {
        let query =
        "SELECT grp, golfername, playername, eup_id, espn_id FROM sp_get_player_names($1) ORDER BY grp, eup_id";
        let query_params = vec![RowValues::Int(event_id as i64)];
        let result = self.exec_general_query(query, query_params, true).await;
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
                            group: row
                                .get("grp")
                                .and_then(|v| v.as_int())
                                .copied()
                                .unwrap_or_default(),
                            // group: row.get("grp").and_then(|v| v.as_ref().map(|s| s.parse::<i64>().unwrap_or_default())).unwrap_or_default(),
                            golfer_name: row
                                .get("golfername")
                                .and_then(|v| v.as_text())
                                .unwrap_or_default()
                                .to_string(),
                            bettor_name: row
                                .get("playername")
                                .and_then(|v| v.as_text())
                                .unwrap_or_default()
                                .to_string(),
                            eup_id: row
                                .get("eup_id")
                                .and_then(|v| v.as_int())
                                .copied()
                                .unwrap_or_default(),
                            espn_id: row
                                .get("espn_id")
                                .and_then(|v| v.as_int())
                                .copied()
                                .unwrap_or_default(),
                            detailed_statistics: Statistic {
                                eup_id: row
                                    .get("eup_id")
                                    .and_then(|v| v.as_int())
                                    .copied()
                                    .unwrap_or_default(),
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
        &self,
        query: &str,
        params: Vec<RowValues>,
        expect_rows: bool, // New parameter to indicate whether rows are expected
    ) -> Result<DatabaseResult<Vec<CustomDbRow>>, sqlx::Error> {
        let mut final_result = DatabaseResult::<Vec<CustomDbRow>>::default();

        if expect_rows {
            // Logic for queries expecting rows
            let exists_result = match &self.pool {
                DbPool::Postgres(pool) => {
                    let mut query = sqlx::query(query);
                    for param in params {
                        query = match param {
                            RowValues::Int(value) => query.bind(value),
                            RowValues::Text(value) => query.bind(value),
                            RowValues::Bool(value) => query.bind(value),
                            RowValues::Timestamp(value) => query.bind(value),
                        };
                    }
                    match query.fetch_all(pool).await {
                        Ok(rows) => {
                            final_result.db_last_exec_state =
                                DatabaseSetupState::QueryReturnedSuccessfully;
                            rows
                            .into_iter()
                            .map(|row| {
                                let column_names =
                                    row.columns().iter().map(|c| c.name().to_string()).collect();
                                let values = row
                                    .columns()
                                    .iter()
                                    .map(|col| {
                                        let type_info = col.type_info().to_string();
                                        let value = match type_info.as_str() {
                                            "INT4" | "INT8" | "BIGINT" | "INTEGER" | "INT" => {
                                                RowValues::Int(row.get::<i64, _>(col.name()))
                                            }
                                            "TEXT" => RowValues::Text(row.get::<String, _>(col.name())),
                                            "BOOL" | "BOOLEAN" => RowValues::Bool(row.get::<bool, _>(col.name())),
                                            "TIMESTAMP" => {
                                                let timestamp: sqlx::types::chrono::NaiveDateTime = row.get(col.name());
                                                RowValues::Timestamp(timestamp.to_string())
                                            }
                                            _ => {
                                                eprintln!("Unknown column type: {}", type_info);
                                                unimplemented!("Unknown column type: {}", type_info)
                                            }
                                        };
                                        value
                                    })
                                    .collect();

                                CustomDbRow {
                                    column_names,
                                    rows: values,
                                }
                            })
                            .collect::<Vec<_>>()
                        }
                        Err(e) => {
                            final_result.db_last_exec_state = DatabaseSetupState::QueryError;
                            final_result.error_message = Some(e.to_string());
                            let z = CustomDbRow {
                                column_names: vec![],
                                rows: vec![],
                            };
                            vec![z]
                        }
                    }
                }
                DbPool::Sqlite(pool) => {
                    let mut query = sqlx::query(query);
                    for param in params {
                        query = match param {
                            RowValues::Int(value) => query.bind(value),
                            RowValues::Text(value) => query.bind(value),
                            RowValues::Bool(value) => query.bind(value),
                            RowValues::Timestamp(value) => query.bind(value),
                        };
                    }
                    match query.fetch_all(pool).await {
                        Ok(rows) => rows
                            .into_iter()
                            .map(|row| {
                                let column_names =
                                    row.columns().iter().map(|c| c.name().to_string()).collect();
                                let values = row
                                    .columns()
                                    .iter()
                                    .map(|col| {
                                        let value = match col.name() {
                                            "INTEGER" => {
                                                RowValues::Int(row.get::<i64, _>(col.name()))
                                            }
                                            "TEXT" => {
                                                RowValues::Text(row.get::<String, _>(col.name()))
                                            }
                                            "BOOLEAN" => {
                                                RowValues::Bool(row.get::<bool, _>(col.name()))
                                            }
                                            _ => unimplemented!(),
                                        };
                                        value
                                    })
                                    .collect();

                                CustomDbRow {
                                    column_names,
                                    rows: values,
                                }
                            })
                            .collect::<Vec<_>>(),
                        Err(e) => return Err(e),
                    }
                }
            };

            final_result.return_result = exists_result;
        } else {
            // Logic for queries not expecting rows (e.g., CREATE TABLE)
            let exec_result: Result<Vec<CustomDbRow>, sqlx::Error> = match &self.pool {
                DbPool::Postgres(pool) => {
                    let mut query = sqlx::query(query);
                    for param in params {
                        query = match param {
                            RowValues::Int(value) => query.bind(value),
                            RowValues::Text(value) => query.bind(value),
                            RowValues::Bool(value) => query.bind(value),
                            RowValues::Timestamp(value) => query.bind(value),
                        };
                    }
                    query.execute(pool).await.map(|_| vec![])
                }
                DbPool::Sqlite(pool) => {
                    let mut query = sqlx::query(query);
                    for param in params {
                        query = match param {
                            RowValues::Int(value) => query.bind(value),
                            RowValues::Text(value) => query.bind(value),
                            RowValues::Bool(value) => query.bind(value),
                            RowValues::Timestamp(value) => query.bind(value),
                        };
                    }
                    query.execute(pool).await.map(|_| vec![])
                }
            };

            match exec_result {
                Ok(_) => {
                    final_result.return_result = vec![];
                    final_result.db_last_exec_state = DatabaseSetupState::QueryReturnedSuccessfully;
                }
                Err(e) => {
                    final_result.db_last_exec_state = DatabaseSetupState::QueryError;
                    final_result.error_message = Some(e.to_string());
                }
            }
        }

        Ok(final_result)
    }

    // fn row_to_map<R, T>(row: R) -> HashMap<String, Option<T>>
    // where
    //     R: Row,
    //     T: for<'a> sqlx::Decode<'a, R::Database> + sqlx::Type<R::Database> + std::fmt::Debug,
    //     usize: ColumnIndex<R>,
    // {
    //     let mut map = HashMap::new();
    //     for (idx, col) in row.columns().iter().enumerate() {
    //         let val: Option<T> = row.try_get(idx).ok();
    //         map.insert(col.name().to_string(), val);
    //     }
    //     map
    // }

    // fn create_error_result<E>(&mut self, e: Error) -> DatabaseResult<Vec<Row>>
    // where
    //     E: std::fmt::Display,
    // {
    //     let cause = match e.as_db_error() {
    //         Some(de) => de.to_string(),
    //         None => e.to_string(),
    //     };
    //     let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), cause);
    //     let mut result = DatabaseResult::<Vec<Row>>::default();

    //     result.db_last_exec_state = self.map_pg_errors_to_setup_state(&e);
    //     result.error_message = Some(emessage);
    //     result
    // }
}

#[cfg(test)]
mod tests {
    use std::{env, vec};

    // use sqlx::query;
    use tokio::runtime::Runtime;

    use super::*;

    #[test]
    fn test_pg_create_and_populate_table() {
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

            let pg_objs = vec![MissingDbObjects {
                missing_object: "test".to_string(),
            }];

            // create a test table
            let pg_create_result = postgres_db
                .create_tables(pg_objs, CheckType::Table, TABLE_DDL)
                .await
                .unwrap();

            assert_eq!(
                pg_create_result.db_last_exec_state,
                DatabaseSetupState::QueryReturnedSuccessfully
            );
            assert_eq!(pg_create_result.return_result, String::default());

            const TABLE_DDL_SYNTAX_ERR: &[(&str, &str, &str, &str)] = &[(
                "test",
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
                missing_object: "test".to_string(),
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
                .exec_general_query(query, params, false)
                .await
                .unwrap();

            let query = "INSERT INTO test (espn_id, name) VALUES ($1, $2)";
            let params: Vec<RowValues> = vec![
                RowValues::Int(123456),
                RowValues::Text("test name".to_string()),
            ];
            // params.push("test name".to_string());
            let _ = postgres_db
                .exec_general_query(query, params, false)
                .await
                .unwrap();

            let query = "SELECT * FROM test";
            let params: Vec<RowValues> = vec![];
            let result = postgres_db
                .exec_general_query(query, params, true)
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
                        RowValues::Timestamp("2021-09-01 00:00:00".to_string()), // this col won't match, obviously, since who knows when we're running the test
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

            let cols_to_actually_check = vec!["espn_id", "name"];

            for (index, row) in result.return_result.iter().enumerate() {
                let left: Vec<RowValues> = row
                    .column_names
                    .iter()
                    .zip(&row.rows) // Pair column names with corresponding row values
                    .filter(|(col_name, _)| cols_to_actually_check.contains(&col_name.as_str())) // other two cols won't match
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
            let _ = postgres_db
                .exec_general_query(query, params, false)
                .await
                .unwrap();

            // // table already created, this should fail
            // let x = db
            //     .create_tbl(TABLE_DDL, "test".to_string(), CheckType::Table)
            //     .await
            //     .unwrap();

            // //TODO: Failing
            // assert_eq!(x.db_last_exec_state, DatabaseSetupState::QueryError);
            // assert_eq!(x.return_result, String::default());

            // // but table should exist
            // let result = db
            //     .check_obj_exists(
            //         "test",
            //         &CheckType::Table,
            //         "test_check_obj_exists_constraint",
            //     )
            //     .await;
            // assert!(result.is_ok());
            // let db_result = result.unwrap();
            // assert_eq!(db_result.db_object_name, "player");

            // // and now we should be able to delete it
            // let xa = db
            //     .delete_table(TABLE_DDL, "test".to_string(), CheckType::Table)
            //     .await
            //     .unwrap();

            // assert_eq!(
            //     xa.db_last_exec_state,
            //     DatabaseSetupState::QueryReturnedSuccessfully
            // );
            // assert_eq!(x.return_result, String::default());

            // // table should be gone
            // let result = db
            //     .check_obj_exists(
            //         "test",
            //         &CheckType::Table,
            //         "test_check_obj_exists_constraint",
            //     )
            //     .await;
            // assert!(result.is_ok());
            // let db_result = result.unwrap();
            // assert_eq!(db_result.db_object_name, "player");
        });
    }

    // #[test]
    // fn test_check_obj_exists_constraint_sqlite() {
    //     let rt = Runtime::new().unwrap();
    //     rt.block_on(async {
    //         // env::var("DB_USER") = Ok("postgres".to_string());

    //         const TABLE_DDL: &[(&str, &str, &str, &str)] = &[(
    //             "test",
    //             "CREATE TABLE IF NOT EXISTS -- drop table event cascade
    //                 test (
    //                 event_id BIGSERIAL NOT NULL PRIMARY KEY,
    //                 espn_id BIGINT NOT NULL,
    //                 name TEXT NOT NULL,
    //                 ins_ts TIMESTAMP NOT NULL DEFAULT now()
    //                 );",
    //             "",
    //             "",
    //         )];

    //         dotenv::dotenv().unwrap();

    //         let mut db_pwd = env::var("DB_PASSWORD").unwrap();
    //         if db_pwd == "/secrets/db_password" {
    //             // open the file and read the contents
    //             let contents = std::fs::read_to_string("/secrets/db_password")
    //                 .unwrap_or("tempPasswordWillbeReplacedIn!AdminPanel".to_string());
    //             // set the password to the contents of the file
    //             db_pwd = contents.trim().to_string();
    //         }
    //         let mut cfg = deadpool_postgres::Config::new();
    //         cfg.dbname = Some(env::var("DB_NAME").unwrap());
    //         cfg.host = Some(env::var("DB_HOST").unwrap());
    //         cfg.port = Some(env::var("DB_PORT").unwrap().parse::<u16>().unwrap());
    //         cfg.user = Some(env::var("DB_USER").unwrap());
    //         cfg.password = Some(db_pwd);

    //         let postgres_dbconfig: DbConfigAndPool =
    //             DbConfigAndPool::new(cfg, DatabaseType::Postgres).await;
    //         let mut postgres_db = Db::new(postgres_dbconfig).unwrap();

    //         let mut sqlite_config = deadpool_postgres::Config::new();
    //         sqlite_config.dbname = Some("/tmp/test.db".to_string());

    //         let sqlitex = DbConfigAndPool::new(sqlite_config, DatabaseType::Sqlite).await;
    //         let mut sqlite_db = Db::new(sqlitex).unwrap();

    //         let pg_objs = vec![MissingDbObjects {
    //             missing_object: "test".to_string(),
    //         }];
    //         let sqlite_objs = pg_objs.clone();
    //         // create a test table
    //         let pg_create_result = postgres_db
    //             .create_tables(pg_objs, CheckType::Table, TABLE_DDL)
    //             .await
    //             .unwrap();

    //         let sqlite_create_result = sqlite_db
    //             .create_tables(sqlite_objs, CheckType::Table, TABLE_DDL)
    //             .await
    //             .unwrap();

    //         assert_eq!(
    //             pg_create_result.db_last_exec_state,
    //             DatabaseSetupState::QueryReturnedSuccessfully
    //         );
    //         assert_eq!(pg_create_result.return_result, String::default());

    //         assert_eq!(
    //             sqlite_create_result.db_last_exec_state,
    //             DatabaseSetupState::QueryReturnedSuccessfully
    //         );
    //         assert_eq!(sqlite_create_result.return_result, String::default());

    //         // // table already created, this should fail
    //         // let x = db
    //         //     .create_tbl(TABLE_DDL, "test".to_string(), CheckType::Table)
    //         //     .await
    //         //     .unwrap();

    //         // //TODO: Failing
    //         // assert_eq!(x.db_last_exec_state, DatabaseSetupState::QueryError);
    //         // assert_eq!(x.return_result, String::default());

    //         // // but table should exist
    //         // let result = db
    //         //     .check_obj_exists(
    //         //         "test",
    //         //         &CheckType::Table,
    //         //         "test_check_obj_exists_constraint",
    //         //     )
    //         //     .await;
    //         // assert!(result.is_ok());
    //         // let db_result = result.unwrap();
    //         // assert_eq!(db_result.db_object_name, "player");

    //         // // and now we should be able to delete it
    //         // let xa = db
    //         //     .delete_table(TABLE_DDL, "test".to_string(), CheckType::Table)
    //         //     .await
    //         //     .unwrap();

    //         // assert_eq!(
    //         //     xa.db_last_exec_state,
    //         //     DatabaseSetupState::QueryReturnedSuccessfully
    //         // );
    //         // assert_eq!(x.return_result, String::default());

    //         // // table should be gone
    //         // let result = db
    //         //     .check_obj_exists(
    //         //         "test",
    //         //         &CheckType::Table,
    //         //         "test_check_obj_exists_constraint",
    //         //     )
    //         //     .await;
    //         // assert!(result.is_ok());
    //         // let db_result = result.unwrap();
    //         // assert_eq!(db_result.db_object_name, "player");
    //     });
    // }
}
