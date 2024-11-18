// use async_trait::async_trait;
use sqlx::{
    self,
    postgres:: PgRow,
    
    sqlite:: SqliteRow,
    Column, ColumnIndex,  Pool, Row,
};
use std::collections::HashMap;

use crate::model::{ResultStatus, Scores, Statistic};

pub enum DatabaseType {
    Postgres,
    Sqlite,
}

pub enum ObjType {
    Table,
    Constraint,
}

pub enum DbPool {
    Postgres(Pool<sqlx::Postgres>),
    Sqlite(Pool<sqlx::Sqlite>),
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

pub struct DbConfigAndPool {
    pool: DbPool,
    db_type: DatabaseType,
}

trait PostgresParam: for<'a> sqlx::Encode<'a, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> {}
impl<T> PostgresParam for T where
    T: for<'a> sqlx::Encode<'a, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>
{
}

trait SqliteParam: for<'a> sqlx::Encode<'a, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> {}
impl<T> SqliteParam for T where T: for<'a> sqlx::Encode<'a, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> {}

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

    pub async fn obj_exists(&self, obj_name: &str, typ: ObjType) -> Result<bool, sqlx::Error> {
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
    pub async fn run_query<P>(
        &self,
        query: &str,
        params: Vec<P>,
    ) -> Result<DatabaseResult<Vec<HashMap<String, Option<String>>>>, sqlx::Error>
    where
        P: PostgresParam + SqliteParam,
    {
        let mut final_result: DatabaseResult<Vec<HashMap<String, Option<String>>>> =
            DatabaseResult::<Vec<HashMap<String, Option<String>>>>::default();

        let exists_result: Vec<HashMap<String, Option<String>>> = match &self.pool {
            DbPool::Postgres(pool) => {
                let mut query = sqlx::query(query);
                for param in params {
                    query = query.bind(param);
                }
                match query.fetch_all(pool).await {
                    Ok(rows) => {
                        let result = rows
                            .into_iter()
                            .map(|row: PgRow| Self::row_to_map(row))
                            .collect::<Vec<_>>();
                        result
                    }

                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            DbPool::Sqlite(pool) => {
                let mut query = sqlx::query(query);
                for param in params {
                    query = query.bind(param);
                }
                match query.fetch_all(pool).await {
                    Ok(rows) => {
                        let result = rows
                            .into_iter()
                            .map(|row: SqliteRow| Self::row_to_map(row))
                            .collect::<Vec<_>>();
                        result
                    }

                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        };
        final_result.return_result = exists_result;
        Ok(final_result)
    }

    fn row_to_map<R>(row: R) -> HashMap<String, Option<String>>
    where
        R: Row,
        String: for<'a> sqlx::Decode<'a, R::Database>,
        std::string::String: sqlx::Type<<R as sqlx::Row>::Database>,
        usize: ColumnIndex<R>,
    {
        let mut map = HashMap::new();
        for (idx, col) in row.columns().iter().enumerate() {
            let val: Option<String> = row.try_get(idx).ok();
            map.insert(col.name().to_string(), val);
        }
        map
    }

    pub async fn get_title_from_db(
        &self,
        event_id: i32,
    ) -> Result<DatabaseResult<String>, Box<dyn std::error::Error>> {
        let query = "SELECT eventname FROM sp_get_event_name($1)";
        let params = vec![&event_id];
        let result = self.run_query(query, params).await;

        let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();

        match result {
            Ok(r) => {
                if r.db_last_exec_state == DatabaseSetupState::QueryReturnedSuccessfully {
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
        let result = self.run_query(query, query_params).await;
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
}
