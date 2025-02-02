use serde_json::Value;
// use sqlx_middleware::db::{ConfigAndPool, DatabaseType, Db, QueryState};
use sql_middleware::{
    middleware::{
        AnyConnWrapper,
        ConfigAndPool,
        DatabaseType,
        MiddlewarePool,
        QueryAndParams,
        RowValues,
    },
    sqlite_convert_params,
    SqlMiddlewareDbError,
};


pub async fn db_prefill(
    json1: &Value,
    config_and_pool: &ConfigAndPool,
    db_type: DatabaseType
) -> Result<(), SqlMiddlewareDbError> {
    let json2 = json1.clone();

    let json = json2;
    let pool = config_and_pool.pool.get().await.map_err(SqlMiddlewareDbError::from)?;
    let conn = MiddlewarePool::get_connection(pool).await.map_err(SqlMiddlewareDbError::from)?;

    Ok(
        (match db_type {
            DatabaseType::Sqlite => {
                // let conn = conn.lock().unwrap();
                conn.interact_sync(move |wrapper_fn| {
                    match wrapper_fn {
                        AnyConnWrapper::Sqlite(sql_conn) => {
                            if cfg!(debug_assertions) {
                                #[allow(unused_variables)]
                                let pretty_json = serde_json::to_string_pretty(&json).unwrap();
                            }

                            let data = json.as_array().unwrap();

                            let tx = sql_conn.transaction()?;
                            for datum in data {
                                let query_and_params_vec = QueryAndParams {
                                    query: "SELECT * FROM event WHERE event_id = ?1 AND year = ?2;".to_string(),
                                    params: vec![
                                        RowValues::Int(datum["event"].as_i64().unwrap()),
                                        RowValues::Int(datum["year"].as_i64().unwrap())
                                    ],
                                };

                                {
                                    let converted_params = sqlite_convert_params(
                                        &query_and_params_vec.params
                                    )?;
                                    let result_set = {
                                        let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                                        let rs = sql_middleware::sqlite_build_result_set(
                                            &mut stmt,
                                            &converted_params
                                        )?;
                                        rs
                                    };
                                    if result_set.results.len() == 0 {
                                        let query_and_params_vec = QueryAndParams {
                                            query: "INSERT INTO event (name, espn_id, year) VALUES(?1, ?2, ?3);".to_string(),
                                            params: vec![
                                                RowValues::Text(
                                                    datum["name"].as_str().unwrap().to_string()
                                                ),
                                                RowValues::Int(datum["event"].as_i64().unwrap()),
                                                RowValues::Int(datum["year"].as_i64().unwrap())
                                            ],
                                        };
                                        let converted_params =
                                            sql_middleware::sqlite_convert_params_for_execute(
                                                query_and_params_vec.params
                                            )?;

                                        let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                                        stmt.execute(converted_params)?;

                                        let query_and_params_vec = QueryAndParams {
                                            query: "SELECT * FROM event WHERE espn_id = ?1 AND year = ?2;".to_string(),
                                            params: vec![
                                                RowValues::Int(datum["event"].as_i64().unwrap()),
                                                RowValues::Int(datum["year"].as_i64().unwrap())
                                            ],
                                        };
                                        let converted_params =
                                            sql_middleware::sqlite_convert_params(
                                                &query_and_params_vec.params
                                            )?;
                                        let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                                        let result_set = {
                                            let rs = sql_middleware::sqlite_build_result_set(
                                                &mut stmt,
                                                &converted_params
                                            )?;
                                            rs
                                        };
                                        // we better have just this one row in there
                                        assert_eq!(result_set.results.len(), 1);

                                        let data_to_fill =
                                            datum["data_to_fill_if_event_and_year_missing"]
                                                .as_array()
                                                .unwrap();
                                        for data in data_to_fill {
                                            let bettors = data["bettors"].as_array().unwrap();
                                            for bettor in bettors {
                                                let query_and_params_vec = QueryAndParams {
                                                    query: "INSERT INTO bettor (name) SELECT ?1 WHERE NOT EXISTS (SELECT 1 from bettor where name = ?1);".to_string(),
                                                    params: vec![
                                                        RowValues::Text(
                                                            bettor.as_str().unwrap().to_string()
                                                        )
                                                    ],
                                                };
                                                let converted_params =
                                                    sql_middleware::sqlite_convert_params_for_execute(
                                                        query_and_params_vec.params
                                                    )?;

                                                let mut stmt = tx.prepare(
                                                    &query_and_params_vec.query
                                                )?;
                                                stmt.execute(converted_params)?;
                                            }
                                            let golfers = data["golfers"].as_array().unwrap();
                                            for golfer in golfers {
                                                let query_and_params_vec = QueryAndParams {
                                                    query: "INSERT INTO golfer (name, espn_id) SELECT ?1, ?2 WHERE NOT EXISTS (SELECT 1 from golfer where espn_id = ?2);".to_string(),
                                                    params: vec![
                                                        RowValues::Text(
                                                            golfer["name"]
                                                                .as_str()
                                                                .unwrap()
                                                                .to_string()
                                                        ),
                                                        RowValues::Int(
                                                            golfer["espn_id"].as_i64().unwrap()
                                                        )
                                                    ],
                                                };
                                                let converted_params =
                                                    sql_middleware::sqlite_convert_params_for_execute(
                                                        query_and_params_vec.params
                                                    )?;

                                                let mut stmt = tx.prepare(
                                                    &query_and_params_vec.query
                                                )?;
                                                stmt.execute(converted_params)?;
                                            }
                                            let event_user_player = data["event_user_player"]
                                                .as_array()
                                                .unwrap();
                                            for event_user_player in event_user_player {
                                                let query_and_params_vec = QueryAndParams {
                                                    query: {
                                                        let mut query =
                                                            "INSERT INTO event_user_player (event_id, user_id, golfer_id)".to_string();
                                                        query.push_str(
                                                            " select (select event_id from event where espn_id = ?1),"
                                                        );
                                                        query.push_str(
                                                            "(select user_id from bettor where name = ?2),"
                                                        );
                                                        query.push_str(
                                                            "(select golfer_id from golfer where espn_id = ?3);"
                                                        );
                                                        query
                                                    },
                                                    params: vec![
                                                        RowValues::Int(
                                                            datum["event"].as_i64().unwrap()
                                                        ),
                                                        RowValues::Text(
                                                            event_user_player["bettor"]
                                                                .as_str()
                                                                .unwrap()
                                                                .to_string()
                                                        ),
                                                        RowValues::Int(
                                                            event_user_player["golfer_espn_id"]
                                                                .as_i64()
                                                                .unwrap()
                                                        )
                                                    ],
                                                };
                                                let converted_params =
                                                    sql_middleware::sqlite_convert_params_for_execute(
                                                        query_and_params_vec.clone().params
                                                    )?;

                                                let mut stmt = tx.prepare(
                                                    &query_and_params_vec.query
                                                )?;
                                                let x = stmt.execute(converted_params);
                                                match x {
                                                    Ok(_) => {}
                                                    Err(e) => {
                                                        println!(
                                                            "event_id {:?}, user_id {:?}, golfer_id {:?}, qry: {:?}, err {:?}",
                                                            query_and_params_vec.clone().params[0],
                                                            query_and_params_vec.clone().params[1],
                                                            query_and_params_vec.clone().params[2],
                                                            stmt.expanded_sql(),
                                                            e
                                                        );
                                                        return Err(e.into());
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        let x = format!(
                                            "Event {} and year {} already exist in the db. Skipping db prefill.",
                                            datum["event"].as_i64().unwrap(),
                                            datum["year"].as_i64().unwrap()
                                        );
                                        println!("{}", x.to_string());
                                    }
                                }
                            }
                            tx.commit()?;
                            Ok::<_, SqlMiddlewareDbError>(())
                        }
                        _ => Err(SqlMiddlewareDbError::Other("Unexpected database type".into())),
                    }
                }).await
                // .map_err(|e| format!("Error executing query: {:?}", e))
            }
            _ => unimplemented!(),
        })??
    )
}
