use sql_middleware::{
    SqlMiddlewareDbError,
    middleware::{
        ConfigAndPool, CustomDbRow, MiddlewarePool, MiddlewarePoolConnection, QueryAndParams,
    },
    postgres_build_result_set, sqlite_build_result_set,
};

use crate::model::CheckType;

/// # Errors
///
/// Will return `Err` if the database query fails
pub async fn test_is_db_setup(
    config_and_pool: &ConfigAndPool,
    check_type: &CheckType,
) -> Result<Vec<CustomDbRow>, Box<dyn std::error::Error>> {
    let pool = config_and_pool.pool.get().await?;
    let sconn = MiddlewarePool::get_connection(pool).await?;

    let query = match &sconn {
        MiddlewarePoolConnection::Postgres(_xx) => {
            if let CheckType::Table = check_type {
                include_str!("../sql/schema/postgres/0x_tables_exist.sql")
            } else {
                return Ok(vec![]);
            }
        }
        MiddlewarePoolConnection::Sqlite(_xx) => {
            if let CheckType::Table = check_type {
                include_str!("../sql/schema/sqlite/0x_tables_exist.sql")
            } else {
                return Ok(vec![]);
            }
        }
    };

    let query_and_params = QueryAndParams {
        query: query.to_string(),
        params: vec![],
    };

    let res = match sconn {
        MiddlewarePoolConnection::Postgres(mut xx) => {
            let tx = xx.transaction().await?;

            let result_set = {
                let stmt = tx.prepare(&query_and_params.query).await?;

                postgres_build_result_set(&stmt, &[], &tx).await?
            };
            tx.commit().await?;
            Ok::<_, SqlMiddlewareDbError>(result_set)
        }
        MiddlewarePoolConnection::Sqlite(xx) => {
            xx.interact(move |xxx| {
                let tx = xxx.transaction()?;
                let result_set = {
                    let mut stmt = tx.prepare(&query_and_params.query)?;

                    sqlite_build_result_set(&mut stmt, &[])?
                };
                tx.commit()?;
                Ok::<_, SqlMiddlewareDbError>(result_set)
            })
            .await?
        }
    }?;

    Ok(res.results)
}

/// # Errors
///
/// Will return `Err` if the database query fails
pub async fn create_tables(
    config_and_pool: &ConfigAndPool,
    check_type: &CheckType,
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = config_and_pool.pool.get().await?;
    let sconn = MiddlewarePool::get_connection(pool).await?;

    let query = if let CheckType::Table = *check_type {
        match &sconn {
            MiddlewarePoolConnection::Postgres(_xx) => {
                if let CheckType::Table = check_type {
                    [
                        include_str!("../sql/schema/postgres/00_event.sql"),
                        include_str!("../sql/schema/postgres/02_golfer.sql"),
                        include_str!("../sql/schema/postgres/03_bettor.sql"),
                        include_str!("../sql/schema/postgres/04_event_user_player.sql"),
                        include_str!("../sql/schema/postgres/05_eup_statistic.sql"),
                    ]
                    .join("\n")
                } else {
                    return Ok(());
                }
            }
            MiddlewarePoolConnection::Sqlite(_xx) => {
                if let CheckType::Table = check_type {
                    [
                        include_str!("../sql/schema/sqlite/00_event.sql"),
                        include_str!("../sql/schema/sqlite/02_golfer.sql"),
                        include_str!("../sql/schema/sqlite/03_bettor.sql"),
                        include_str!("../sql/schema/sqlite/04_event_user_player.sql"),
                        include_str!("../sql/schema/sqlite/05_eup_statistic.sql"),
                    ]
                    .join("\n")
                } else {
                    return Ok(());
                }
            }
        }
    } else {
        return Ok(());
    };

    crate::model::execute_batch_sql(config_and_pool, &query).await?;

    Ok(())
}
