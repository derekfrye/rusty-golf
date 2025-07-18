use serde_json::Value;
// use sqlx_middleware::db::{ConfigAndPool, DatabaseType, Db, QueryState};
use sql_middleware::{
    SqlMiddlewareDbError, SqliteParamsExecute, SqliteParamsQuery, convert_sql_params,
    middleware::{
        AnyConnWrapper, ConfigAndPool, ConversionMode, DatabaseType, MiddlewarePool,
        QueryAndParams, RowValues,
    },
};

pub async fn db_prefill(
    json1: &Value,
    config_and_pool: &ConfigAndPool,
    db_type: DatabaseType,
) -> Result<(), SqlMiddlewareDbError> {
    let json2 = json1.clone();

    let json = json2;
    let pool = config_and_pool.pool.get().await?;
    let conn = MiddlewarePool::get_connection(pool).await?;

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
                                    query: "SELECT * FROM event WHERE espn_id = ?1 AND year = ?2;".to_string(),
                                    params: vec![
                                        RowValues::Int(datum["event"].as_i64().unwrap()),
                                        RowValues::Int(datum["year"].as_i64().unwrap())
                                    ],
                                };

                                {
                                    let converted_params = convert_sql_params::<SqliteParamsQuery>(
                                        &query_and_params_vec.params,
                                        ConversionMode::Query
                                    )?;

                                    let result_set = {
                                        let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                                        sql_middleware::sqlite_build_result_set(
                                            &mut stmt,
                                            &converted_params.0
                                        )?
                                    };
                                    if result_set.results.is_empty() {
                                        let query_and_params_vec = QueryAndParams {
                                            query: "INSERT INTO event (name, espn_id, year, score_view_step_factor) VALUES(?1, ?2, ?3, ?4);".to_string(),
                                            params: vec![
                                                RowValues::Text(
                                                    datum["name"].as_str().unwrap().to_string()
                                                ),
                                                RowValues::Int(datum["event"].as_i64().unwrap()),
                                                RowValues::Int(datum["year"].as_i64().unwrap()),
                                                RowValues::Float(
                                                    datum["score_view_step_factor"]
                                                        .as_f64()
                                                        .unwrap()
                                                )
                                            ],
                                        };
                                        let converted_params =
                                            convert_sql_params::<SqliteParamsExecute>(
                                                &query_and_params_vec.params,
                                                ConversionMode::Execute
                                            )?;

                                        let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                                        stmt.execute(converted_params.0)?;

                                        let query_and_params_vec = QueryAndParams {
                                            query: "SELECT * FROM event WHERE espn_id = ?1 AND year = ?2;".to_string(),
                                            params: vec![
                                                RowValues::Int(datum["event"].as_i64().unwrap()),
                                                RowValues::Int(datum["year"].as_i64().unwrap())
                                            ],
                                        };
                                        let converted_params =
                                            convert_sql_params::<SqliteParamsQuery>(
                                                &query_and_params_vec.params,
                                                ConversionMode::Query
                                            )?;
                                        let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                                        let result_set = {
                                            sql_middleware::sqlite_build_result_set(
                                                &mut stmt,
                                                &converted_params.0
                                            )?
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
                                                    convert_sql_params::<SqliteParamsExecute>(
                                                        &query_and_params_vec.params,
                                                        ConversionMode::Execute
                                                    )?;

                                                let mut stmt = tx.prepare(
                                                    &query_and_params_vec.query
                                                )?;
                                                stmt.execute(converted_params.0)?;
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
                                                    convert_sql_params::<SqliteParamsExecute>(
                                                        &query_and_params_vec.params,
                                                        ConversionMode::Execute
                                                    )?;

                                                let mut stmt = tx.prepare(
                                                    &query_and_params_vec.query
                                                )?;
                                                stmt.execute(converted_params.0)?;
                                            }
                                            let event_user_player = data["event_user_player"]
                                                .as_array()
                                                .unwrap();
                                            for event_user_player in event_user_player {
                                                let mut params = vec![
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
                                                    ];
                                                let mut query_columns = "(event_id, user_id, golfer_id".to_string();
                                                let mut query_values = " select (select event_id from event where espn_id = ?1),".to_string();
                                                query_values.push_str("(select user_id from bettor where name = ?2),");
                                                query_values.push_str("(select golfer_id from golfer where espn_id = ?3)");
                                                // Check if score_view_step_factor is present in JSON
                                                if event_user_player.get("score_view_step_factor").is_some() {
                                                    query_columns.push_str(", score_view_step_factor");
                                                    query_values.push_str(", ?4");
                                                    params.push(RowValues::Float(
                                                        event_user_player["score_view_step_factor"]
                                                            .as_f64()
                                                            .unwrap()
                                                    ));
                                                } else {
                                                    query_columns.push_str(", score_view_step_factor");
                                                    query_values.push_str(", NULL");
                                                }
                                                query_columns.push(')');
                                                query_values.push(';');
                                                let query_and_params_vec = QueryAndParams {
                                                    query: format!("INSERT INTO event_user_player {query_columns}{query_values}"),
                                                    params,
                                                };
                                                let converted_params =
                                                    convert_sql_params::<SqliteParamsExecute>(
                                                        &query_and_params_vec.params,
                                                        ConversionMode::Execute
                                                    )?;

                                                let mut stmt = tx.prepare(
                                                    &query_and_params_vec.query
                                                )?;
                                                let x = stmt.execute(converted_params.0);
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
                                        println!("{x}");
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
    })?
}
