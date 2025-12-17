#![allow(dead_code)]
use std::time::{SystemTime, UNIX_EPOCH};

use rusty_golf::args::CleanArgs;
use sql_middleware::SqlMiddlewareDbError;
use sql_middleware::middleware::{
    ConfigAndPool, DatabaseType, MiddlewarePoolConnection, ResultSet, RowValues, SqliteOptions,
};

pub struct TestContext {
    pub config_and_pool: ConfigAndPool,
    pub args: CleanArgs,
}

pub async fn setup_test_context(fixture_sql: &str) -> Result<TestContext, SqlMiddlewareDbError> {
    let db_name = format!(
        "file:test_db_{}?mode=memory&cache=shared",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time went backwards")
            .as_nanos()
    );

    let sqlite_options = SqliteOptions::new(db_name.clone());
    let config_and_pool = ConfigAndPool::new_sqlite(sqlite_options).await?;
    let args = CleanArgs {
        db_type: DatabaseType::Sqlite,
        db_name,
        db_host: None,
        db_port: None,
        db_user: None,
        db_password: None,
        db_startup_script: None,
        db_populate_json: None,
        combined_sql_script: String::new(),
    };

    execute_batch(
        &config_and_pool,
        include_str!("../../src/sql/schema/sqlite/00_table_drop.sql"),
    )
    .await?;

    let schema = [
        include_str!("../../src/sql/schema/sqlite/00_event.sql"),
        include_str!("../../src/sql/schema/sqlite/02_golfer.sql"),
        include_str!("../../src/sql/schema/sqlite/03_bettor.sql"),
        include_str!("../../src/sql/schema/sqlite/04_event_user_player.sql"),
        include_str!("../../src/sql/schema/sqlite/05_eup_statistic.sql"),
    ]
    .join("\n");
    execute_batch(&config_and_pool, &schema).await?;

    execute_batch(&config_and_pool, fixture_sql).await?;

    Ok(TestContext {
        config_and_pool,
        args,
    })
}

async fn execute_batch(
    config_and_pool: &ConfigAndPool,
    sql: &str,
) -> Result<(), SqlMiddlewareDbError> {
    let mut conn = config_and_pool.get_connection().await?;

    conn.execute_batch(sql).await
}

pub trait ConnExt {
    async fn execute_select(
        &mut self,
        query: &str,
        params: &[RowValues],
    ) -> Result<ResultSet, SqlMiddlewareDbError>;

    async fn execute_dml(
        &mut self,
        query: &str,
        params: &[RowValues],
    ) -> Result<usize, SqlMiddlewareDbError>;
}

impl ConnExt for MiddlewarePoolConnection {
    async fn execute_select(
        &mut self,
        query: &str,
        params: &[RowValues],
    ) -> Result<ResultSet, SqlMiddlewareDbError> {
        self.query(query).params(params).select().await
    }

    async fn execute_dml(
        &mut self,
        query: &str,
        params: &[RowValues],
    ) -> Result<usize, SqlMiddlewareDbError> {
        self.query(query).params(params).dml().await
    }
}
