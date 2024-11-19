use crate::model::{ResultStatus, Scores, Statistic};

use ::function_name::named;
use deadpool_postgres::Config;//, Pool, PoolError, Runtime};
// use std::env;
// use tokio_postgres::{Error as PgError, NoTls, Row};

use crate::admin::model::admin_model::MissingDbObjects;

use sqlx::{
    self,
    postgres:: PgRow,
    
    sqlite:: SqliteRow,
    Column, ColumnIndex,  Pool, Row,
};
use std::collections::HashMap;



#[derive(Debug, Clone, )]
pub enum DbPool {
    Postgres(Pool<sqlx::Postgres>),
    Sqlite(Pool<sqlx::Sqlite>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseType {
    Postgres,
    Sqlite,
}

pub enum ObjType {
    Table,
    Constraint,
}

trait PostgresParam: for<'a> sqlx::Encode<'a, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> {}
impl<T> PostgresParam for T where
    T: for<'a> sqlx::Encode<'a, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>
{
}

trait SqliteParam: for<'a> sqlx::Encode<'a, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> {}
impl<T> SqliteParam for T where T: for<'a> sqlx::Encode<'a, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> {}

enum DbQueryResult {
    Postgres(sqlx::postgres::PgQueryResult),
    Sqlite(sqlx::sqlite::SqliteQueryResult),
}

enum DbRow {
    Postgres(sqlx::postgres::PgRow),
    Sqlite(sqlx::sqlite::SqliteRow),
}

enum DbQueryOne<T> {
    Postgres(T),
    Sqlite(T),
}


#[derive(Clone, Debug)]
pub struct DbConfigAndPool {
    pool: DbPool,
    db_type: DatabaseType,
}

impl DbConfigAndPool {
    pub async fn new(config: Config, db_type :DatabaseType,) -> Self {
        if config.dbname.is_none() {
            panic!("dbname is required");
        }

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
                format!(
                    "sqlite://{}",
                    config.dbname.unwrap()
                )
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
                        db_type,
                    },
                    Err(e) => {
                        panic!("Failed to create Postgres pool: {}", e);
                    }
                }
            }
            DatabaseType::Sqlite => {
                let pool_result = sqlx::sqlite::SqlitePoolOptions::new()
                    .connect(&connection_string)
                    .await;
                match pool_result {
                    Ok(pool) => DbConfigAndPool {
                        pool: DbPool::Sqlite(pool),
                        db_type,
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
#[derive(Debug, Clone)]
enum RowValues {
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    // Add other types as needed
}

#[derive(Debug, Clone)]
struct CustomDbRow {
    column_names: Vec<String>,
    rows: Vec<RowValues>,
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
    pub config_and_pool: DbConfigAndPool,
    pub pool: DbPool,
}

impl Db {
    pub fn new(cnf: DbConfigAndPool) -> Result<Self, String> {
        let cnf_clone=cnf.clone();
        Ok(Self {

            config_and_pool: cnf,
            pool: cnf_clone.pool,
        })
    }

    #[named]
    /// Check if tables or constraints are setup.
    pub async fn test_is_db_setup(
        &mut self,
        check_type: &CheckType,
    ) -> Result<Vec<DatabaseResult<String>>, Box<dyn std::error::Error>> {
        let mut dbresults = vec![];

        let query = include_str!("../admin/model/sql/schema/0x_tables_exist.sql");
        let params:Vec<String> = vec![];
        let result = self.exec_general_query::<_, String>(query, params).await;

        let missing_tables= match result {
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
let zz:Vec<_>=        missing_tables.iter().filter_map(|row| {
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
        }).collect();

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
    ) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
        let mut return_result: DatabaseResult<String> = DatabaseResult::<String>::default();
        return_result.db_object_name = function_name!().to_string();

        if check_type == CheckType::Table {
            for table in tables.iter().take_while(|xa| {
                TABLES_AND_DDL
                    .iter()
                    .any(|af| af.0 == xa.missing_object.as_str())
            }) {
                let create_table_attempt = self
                    .create_tbl(
                        TABLES_AND_DDL,
                        table.missing_object.clone(),
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
                        let emessage =
                            format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
                        return_result.db_last_exec_state = DatabaseSetupState::QueryError;
                        return_result.error_message = Some(emessage);
                        return Ok(return_result);
                    }
                }
            }

            return_result.db_last_exec_state = DatabaseSetupState::QueryReturnedSuccessfully;
        } else {
            for table in TABLES_CONSTRAINT_TYPE_CONSTRAINT_NAME_AND_DDL.iter() {
                let create_constraint_attempt = self
                    .create_tbl(
                        TABLES_CONSTRAINT_TYPE_CONSTRAINT_NAME_AND_DDL,
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
                        let emessage =
                            format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
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

    async fn check_obj_exists(&self, obj_name: &str, typ: ObjType) -> Result<bool, sqlx::Error> {
        let check_sql = match &self.pool {
            DbPool::Postgres(_) => match typ {
                ObjType::Table => "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = $1);",
                ObjType::Constraint => "SELECT EXISTS (SELECT FROM information_schema.table_constraints WHERE table_schema = 'public' AND constraint_name = $1);",
            },
            DbPool::Sqlite(_) => match typ {
                ObjType::Table => "SELECT name FROM sqlite_master WHERE type='table' AND name = ?;",
                ObjType::Constraint => "SELECT name FROM sqlite_master WHERE type='constraint' AND name = ?;",
            },
        };
        let exists_result: Result<bool, sqlx::Error> = match &self.pool {
            DbPool::Postgres(pool) => sqlx::query_scalar::<_, bool>(check_sql)
                .bind(obj_name)
                .fetch_one(pool)
                .await
                .map(|result| result),
            DbPool::Sqlite(pool) => sqlx::query_scalar::<_, bool>(check_sql)
                .bind(obj_name)
                .fetch_one(pool)
                .await
                .map(|result| result),
        };
        match exists_result {
            Ok(exists) => {
                return Ok(exists);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    pub async fn get_title_from_db(
        &self,
        event_id: i32,
    ) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
        let query = "SELECT eventname FROM sp_get_event_name($1)";
        let params = vec![&event_id];
        let result = self.exec_general_query(query, params).await;

        let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();

        match result {
            Ok(r) => {
                if r.db_last_exec_state == DatabaseSetupState::QueryReturnedSuccessfully {
                    match r.return_result.get("eventname") {
                        Some(Some(event_name)) => {
                            dbresult.return_result = event_name.clone();
                        }
                        _ => {
                            dbresult.error_message = Some("No event name found".to_string());
                        }
                    }
                    if let Some(Some(event_name)) = r.return_result[0].get("eventname") {
                        dbresult.return_result = event_name.clone();
                    } else {
                        dbresult.error_message = Some("No event name found".to_string());
                    }
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
        &self,
        event_id: i32,
    ) -> Result<DatabaseResult<Vec<Scores>>, Box<dyn std::error::Error>> {
        let query =
        "SELECT grp, golfername, playername, eup_id, espn_id FROM sp_get_player_names($1) ORDER BY grp, eup_id";
        let query_params= vec![&event_id];
        let result = self.exec_general_query(query, query_params).await;
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
                            group: row.get("grp").and_then(|v| v.as_ref().map(|s| s.parse::<i64>().unwrap_or_default())).unwrap_or_default(),
                            golfer_name: row.get("golfername").and_then(|v| v.as_ref().map(|s| s.to_string())).unwrap_or_default(),
                            bettor_name: row.get("playername").and_then(|v| v.as_ref().map(|s| s.to_string())).unwrap_or_default(),
                            eup_id: row.get("eup_id").and_then(|v| v.as_ref().map(|s| s.parse::<i64>().unwrap_or_default())).unwrap_or_default(),
                            espn_id: row.get("espn_id").and_then(|v| v.as_ref().map(|s| s.parse::<i64>().unwrap_or_default())).unwrap_or_default(),
                            detailed_statistics: Statistic {
                                eup_id: row.get("eup_id").and_then(|v| v.as_ref().map(|s| s.parse::<i64>().unwrap_or_default())).unwrap_or_default(),
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

    async fn exec_general_query<P, T>(
        &self,
        query: &str,
        params: Vec<P>,
    ) -> Result<DatabaseResult<Vec<CustomDbRow>>, sqlx::Error>
    where
        P: PostgresParam + SqliteParam,
        T: for<'a> sqlx::Decode<'a, sqlx::Postgres>
            + sqlx::Type<sqlx::Postgres>
            + for<'a> sqlx::Decode<'a, sqlx::Sqlite>
            + sqlx::Type<sqlx::Sqlite>
            + std::fmt::Debug,
    {
        let mut final_result = DatabaseResult::<Vec<CustomDbRow>>::default();

        let exists_result = match &self.pool {
            DbPool::Postgres(pool) => {
                let mut query = sqlx::query(query);
                for param in params {
                    query = query.bind(param);
                }
                let rows: Vec<PgRow> = query.fetch_all(pool).await?;
    
                rows.into_iter().map(|row| {
                    let column_names = row.columns().iter().map(|c| c.name().to_string()).collect();
                    let values = row.columns().iter().map(|col| {
                        let value = match col.name() {
                            "INT4" => RowValues::Int(row.get::<i64, _>(col.name())),
                            "FLOAT8" => RowValues::Float(row.get::<f64, _>(col.name())),
                            "TEXT" => RowValues::Text(row.get::<String, _>(col.name())),
                            "BOOL" => RowValues::Bool(row.get::<bool, _>(col.name())),
                            _ => unimplemented!(),
                        };
                        value
                    }).collect();
    
                    CustomDbRow { column_names, rows: values }
                }).collect::<Vec<_>>()
            }
            DbPool::Sqlite(pool) => {
                let mut query = sqlx::query(query);
                for param in params {
                    query = query.bind(param);
                }
                let rows: Vec<SqliteRow> = query.fetch_all(pool).await?;
    
                rows.into_iter().map(|row| {
                    let column_names = row.columns().iter().map(|c| c.name().to_string()).collect();
                    let values = row.columns().iter().map(|col| {
                        let value = match col.name() {
                            "INTEGER" => RowValues::Int(row.get::<i64, _>(col.name())),
                            "REAL" => RowValues::Float(row.get::<f64, _>(col.name())),
                            "TEXT" => RowValues::Text(row.get::<String, _>(col.name())),
                            "BOOLEAN" => RowValues::Bool(row.get::<bool, _>(col.name())),
                            _ => unimplemented!(),
                        };
                        value
                    }).collect();
    
                    CustomDbRow { column_names, rows: values }
                }).collect::<Vec<_>>()
            }
        };
    
        final_result.return_result = exists_result;
        Ok(final_result)
    }
    

    fn row_to_map<R, T>(row: R) -> HashMap<String, Option<T>>
where
    R: Row,
    T: for<'a> sqlx::Decode<'a, R::Database> + sqlx::Type<R::Database> + std::fmt::Debug,
    usize: ColumnIndex<R>,
{
    let mut map = HashMap::new();
    for (idx, col) in row.columns().iter().enumerate() {
        let val: Option<T> = row.try_get(idx).ok();
        map.insert(col.name().to_string(), val);
    }
    map
}

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
    use std::env;

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

            let x: DbConfigAndPool = DbConfigAndPool::new(cfg).unwrap();

            let mut db = Db::new(x).unwrap();

            // create a test table
            let x = db
                .create_tbl(TABLE_DDL, "test".to_string(), CheckType::Table)
                .await
                .unwrap();

            assert_eq!(
                x.db_last_exec_state,
                DatabaseSetupState::QueryReturnedSuccessfully
            );
            assert_eq!(x.return_result, String::default());

            // table already created, this should fail
            let x = db
                .create_tbl(TABLE_DDL, "test".to_string(), CheckType::Table)
                .await
                .unwrap();

            //TODO: Failing
            assert_eq!(x.db_last_exec_state, DatabaseSetupState::QueryError);
            assert_eq!(x.return_result, String::default());

            // but table should exist
            let result = db
                .check_obj_exists(
                    "test",
                    &CheckType::Table,
                    "test_check_obj_exists_constraint",
                )
                .await;
            assert!(result.is_ok());
            let db_result = result.unwrap();
            assert_eq!(db_result.db_object_name, "player");

            // and now we should be able to delete it
            let xa = db
                .delete_table(TABLE_DDL, "test".to_string(), CheckType::Table)
                .await
                .unwrap();

            assert_eq!(
                xa.db_last_exec_state,
                DatabaseSetupState::QueryReturnedSuccessfully
            );
            assert_eq!(x.return_result, String::default());

            // table should be gone
            let result = db
                .check_obj_exists(
                    "test",
                    &CheckType::Table,
                    "test_check_obj_exists_constraint",
                )
                .await;
            assert!(result.is_ok());
            let db_result = result.unwrap();
            assert_eq!(db_result.db_object_name, "player");
        });
    }
}
