use serde_json::Value;
use sql_middleware::{
    convert_sql_params,
    middleware::{
        AnyConnWrapper, ConfigAndPool, ConversionMode, DatabaseType, MiddlewarePool,
        QueryAndParams, RowValues,
    },
    SqlMiddlewareDbError, SqliteParamsExecute, SqliteParamsQuery,
};

pub async fn db_prefill(
    json: &Value,
    config_and_pool: &ConfigAndPool,
    db_type: DatabaseType,
) -> Result<(), SqlMiddlewareDbError> {
    let pool = config_and_pool.pool.get().await?;
    let conn = MiddlewarePool::get_connection(pool).await?;

    match db_type {
        DatabaseType::Sqlite => prefill_sqlite(conn, json.clone()).await?,
        DatabaseType::Postgres => unimplemented!(),
    }
    Ok(())
}

async fn prefill_sqlite(
    conn: sql_middleware::middleware::MiddlewarePoolConnection,
    json: Value,
) -> Result<(), SqlMiddlewareDbError> {
    conn.interact_sync(move |wrapper_fn| {
        if let AnyConnWrapper::Sqlite(sql_conn) = wrapper_fn {
            if cfg!(debug_assertions) {
                let _pretty_json = serde_json::to_string_pretty(&json).unwrap();
            }

            let data = json.as_array().unwrap();
            let tx = sql_conn.transaction()?;

            for datum in data {
                process_event_datum(&tx, datum)?;
            }

            tx.commit()?;
            Ok(())
        } else {
            Err(SqlMiddlewareDbError::Other(
                "Unexpected database type".into(),
            ))
        }
    })
    .await?
}

fn process_event_datum<T: ConnectionTrait>(
    tx: &T,
    datum: &Value,
) -> Result<(), SqlMiddlewareDbError> {
    let espn_id = datum["event"].as_i64().unwrap();
    let year = datum["year"].as_i64().unwrap();

    if !event_exists(tx, espn_id, year)? {
        insert_event(tx, datum)?;
        let data_to_fill = datum["data_to_fill_if_event_and_year_missing"]
            .as_array()
            .unwrap();
        for data in data_to_fill {
            insert_bettors(tx, data["bettors"].as_array().unwrap())?;
            insert_golfers(tx, data["golfers"].as_array().unwrap())?;
            insert_event_user_players(tx, data["event_user_player"].as_array().unwrap(), datum)?;
        }
    } else {
        println!("Event {espn_id} and year {year} already exist in the db. Skipping db prefill.");
    }
    Ok(())
}

trait ConnectionTrait {
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement, rusqlite::Error>;
}

impl ConnectionTrait for rusqlite::Transaction<'_> {
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement, rusqlite::Error> {
        self.prepare(sql)
    }
}

fn event_exists<T: ConnectionTrait>(
    tx: &T,
    espn_id: i64,
    year: i64,
) -> Result<bool, SqlMiddlewareDbError> {
    let query_and_params = QueryAndParams {
        query: "SELECT * FROM event WHERE espn_id = ?1 AND year = ?2;".to_string(),
        params: vec![RowValues::Int(espn_id), RowValues::Int(year)],
    };
    let converted_params =
        convert_sql_params::<SqliteParamsQuery>(&query_and_params.params, ConversionMode::Query)?;
    let mut stmt = tx.prepare(&query_and_params.query)?;
    let result_set = sql_middleware::sqlite_build_result_set(&mut stmt, &converted_params.0)?;
    Ok(!result_set.results.is_empty())
}

fn insert_event<T: ConnectionTrait>(
    tx: &T,
    datum: &Value,
) -> Result<(), SqlMiddlewareDbError> {
    let query_and_params = QueryAndParams {
        query:
            "INSERT INTO event (name, espn_id, year, score_view_step_factor) VALUES(?1, ?2, ?3, ?4);"
                .to_string(),
        params: vec![
            RowValues::Text(datum["name"].as_str().unwrap().to_string()),
            RowValues::Int(datum["event"].as_i64().unwrap()),
            RowValues::Int(datum["year"].as_i64().unwrap()),
            RowValues::Float(datum["score_view_step_factor"].as_f64().unwrap()),
        ],
    };
    let converted_params = convert_sql_params::<SqliteParamsExecute>(
        &query_and_params.params,
        ConversionMode::Execute,
    )?;
    let mut stmt = tx.prepare(&query_and_params.query)?;
    stmt.execute(converted_params.0)?;
    Ok(())
}

fn insert_bettors<T: ConnectionTrait>(
    tx: &T,
    bettors: &[Value],
) -> Result<(), SqlMiddlewareDbError> {
    for bettor in bettors {
        let query_and_params = QueryAndParams {
            query: "INSERT INTO bettor (name) SELECT ?1 WHERE NOT EXISTS (SELECT 1 from bettor where name = ?1);".to_string(),
            params: vec![RowValues::Text(bettor.as_str().unwrap().to_string())],
        };
        let converted_params = convert_sql_params::<SqliteParamsExecute>(
            &query_and_params.params,
            ConversionMode::Execute,
        )?;
        let mut stmt = tx.prepare(&query_and_params.query)?;
        stmt.execute(converted_params.0)?;
    }
    Ok(())
}

fn insert_golfers<T: ConnectionTrait>(
    tx: &T,
    golfers: &[Value],
) -> Result<(), SqlMiddlewareDbError> {
    for golfer in golfers {
        let query_and_params = QueryAndParams {
            query: "INSERT INTO golfer (name, espn_id) SELECT ?1, ?2 WHERE NOT EXISTS (SELECT 1 from golfer where espn_id = ?2);".to_string(),
            params: vec![
                RowValues::Text(golfer["name"].as_str().unwrap().to_string()),
                RowValues::Int(golfer["espn_id"].as_i64().unwrap()),
            ],
        };
        let converted_params = convert_sql_params::<SqliteParamsExecute>(
            &query_and_params.params,
            ConversionMode::Execute,
        )?;
        let mut stmt = tx.prepare(&query_and_params.query)?;
        stmt.execute(converted_params.0)?;
    }
    Ok(())
}

fn insert_event_user_players<T: ConnectionTrait>(
    tx: &T,
    event_user_players: &[Value],
    datum: &Value,
) -> Result<(), SqlMiddlewareDbError> {
    for event_user_player in event_user_players {
        let mut params = vec![
            RowValues::Int(datum["event"].as_i64().unwrap()),
            RowValues::Text(event_user_player["bettor"].as_str().unwrap().to_string()),
            RowValues::Int(event_user_player["golfer_espn_id"].as_i64().unwrap()),
        ];

        let mut query_columns = "(event_id, user_id, golfer_id".to_string();
        let mut query_values = " select (select event_id from event where espn_id = ?1),"
            .to_string();
        query_values.push_str("(select user_id from bettor where name = ?2),");
        query_values.push_str("(select golfer_id from golfer where espn_id = ?3)");

        if event_user_player.get("score_view_step_factor").is_some() {
            query_columns.push_str(", score_view_step_factor");
            query_values.push_str(", ?4");
            params.push(RowValues::Float(
                event_user_player["score_view_step_factor"]
                    .as_f64()
                    .unwrap(),
            ));
        } else {
            query_columns.push_str(", score_view_step_factor");
            query_values.push_str(", NULL");
        }

        query_columns.push(')');
        query_values.push(';');

        let query = format!("INSERT INTO event_user_player {query_columns}{query_values}");
        let query_and_params = QueryAndParams { query, params };

        let converted_params = convert_sql_params::<SqliteParamsExecute>(
            &query_and_params.params,
            ConversionMode::Execute,
        )?;

        let mut stmt = tx.prepare(&query_and_params.query)?;
        if let Err(e) = stmt.execute(converted_params.0) {
            println!(
                "event_id {:?}, user_id {:?}, golfer_id {:?}, qry: {:?}, err {:?}",
                query_and_params.params[0],
                query_and_params.params[1],
                query_and_params.params[2],
                stmt.expanded_sql(),
                e
            );
            return Err(e.into());
        }
    }
    Ok(())
}

