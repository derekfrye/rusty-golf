// use async_trait::async_trait;
use sqlx::{
    self, postgres::PgQueryResult, sqlite::SqliteQueryResult, Column, Error as sqlxError, Row,
};
use std::{collections::HashMap, result};

pub enum DatabaseType {
    Postgres,
    Sqlite,
}

pub enum ObjType {
    Table,
    Constraint,
}

enum DbPool {
    Postgres(sqlx::Pool<sqlx::Postgres>),
    Sqlite(sqlx::Pool<sqlx::Sqlite>),
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

impl Default for DatabaseResult<()> {
    fn default() -> Self {
        DatabaseResult {
            db_last_exec_state: DatabaseSetupState::NoConnection,
            return_result: (),
            error_message: None,
            db_object_name: "".to_string(),
        }
    }
}

enum DbQueryResult {
    Postgres(sqlx::postgres::PgQueryResult),
    Sqlite(sqlx::sqlite::SqliteQueryResult),
}

enum DbQueryOne<T> {
    Postgres(T),
    Sqlite(T),
}

pub struct DbConfigAndPool {
    pool: DbPool,
    db_type: DatabaseType,
}

impl DbConfigAndPool {
    async fn new(db_type: DatabaseType, connection_str: &str) -> Self {
        match db_type {
            DatabaseType::Postgres => {
                let pool_result = sqlx::postgres::PgPoolOptions::new()
                    .connect(connection_str)
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
                    .connect(connection_str)
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

    pub async fn obj_exists(&self, obj_name: &str, typ:ObjType,) -> Result<bool, sqlx::Error> {
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

    pub async fn object_crud(
        &self,
        obj_name: &str,
        obj_ddl: &str,
        proceed_when_obj_exists: bool,
        typ: ObjType,
    ) -> DatabaseResult<()> {
        let mut result: DatabaseResult<()> = DatabaseResult::default();
        result.db_object_name = obj_name.to_string();
        let exists_result = self.obj_exists(obj_name,typ).await;
        match exists_result {
            Ok(exists) => {
                if exists && proceed_when_obj_exists | !exists {
                    let sql_to_run = obj_ddl;

                    let sql_result: Result<DbQueryResult, sqlx::Error> = match &self.pool {
                        DbPool::Postgres(pool) => sqlx::query(&sql_to_run)
                            .execute(pool)
                            .await
                            .map(DbQueryResult::Postgres),
                        DbPool::Sqlite(pool) => sqlx::query(&sql_to_run)
                            .execute(pool)
                            .await
                            .map(DbQueryResult::Sqlite),
                    };

                    match sql_result {
                        Ok(_) => {
                            result.db_last_exec_state =
                                DatabaseSetupState::QueryReturnedSuccessfully;
                        }
                        Err(e) => {
                            result.db_last_exec_state = DatabaseSetupState::QueryError;
                            result.error_message = Some(e.to_string());
                        }
                    }
                } else {
                    result.db_last_exec_state = DatabaseSetupState::MissingRelations;
                    result.error_message = Some(
                        format!("Object exists and proceed_when_obj_exists is false").to_string(),
                    );
                }
            }
            Err(_) => {
                result.db_last_exec_state = DatabaseSetupState::NoConnection;
                result.error_message = Some("Error checking table existence".to_string());
            }
        }

        result
    }

    async fn run_query(&self, query: &str) -> Vec<HashMap<String, Option<String>>> {
        match &self.pool {
            DbPool::Postgres(pool) => {
                let rows_result = sqlx::query(query).fetch_all(pool).await;
                match rows_result {
                    Ok(rows) => rows
                        .into_iter()
                        .map(|row| {
                            let mut map = HashMap::new();
                            for (idx, col) in row.columns().iter().enumerate() {
                                let val: Option<String> = row.try_get(idx).ok();
                                map.insert(col.name().to_string(), val);
                            }
                            map
                        })
                        .collect(),
                    Err(e) => {
                        println!("Failed to run query: {}", e);
                        Vec::new()
                    }
                }
            }
            DbPool::Sqlite(pool) => {
                let rows_result = sqlx::query(query).fetch_all(pool).await;
                match rows_result {
                    Ok(rows) => rows
                        .into_iter()
                        .map(|row| {
                            let mut map = HashMap::new();
                            for (idx, col) in row.columns().iter().enumerate() {
                                let val: Option<String> = row.try_get(idx).ok();
                                map.insert(col.name().to_string(), val);
                            }
                            map
                        })
                        .collect(),
                    Err(e) => {
                        println!("Failed to run query: {}", e);
                        Vec::new()
                    }
                }
            }
        }
    }
}
