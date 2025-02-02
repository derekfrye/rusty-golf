use serde_json::Value;
// use sqlx_middleware::db::{ConfigAndPool, DatabaseType, Db, QueryState};
use sql_middleware::{
    middleware::{
        ConfigAndPool,
        MiddlewarePool,
        MiddlewarePoolConnection,
        QueryAndParams,
        RowValues,
    },
    sqlite_convert_params,
    SqlMiddlewareDbError,
};

/// format we have is this:
/// [{ "event": <int>, "year": <int>, "name": "value",  "data_to_fill_if_event_and_year_missing": [
/// { "bettors": [{"PlayerName", "PlayerName2", "PlayerName3"...}]
/// , "golfers": [{"name": "Firstname Lastname", "espn_id": <int>}, {"name": "Firstname Lastname", "espn_id": <int>}, ...]
/// , "event_user_player": [{"bettor": "PlayerName", "golfer_espn_id": <int>}, {"bettor": "PlayerName", "golfer_espn_id": <int>}, ...]
/// }]}]
pub async fn db_prefill(
    json1: &Value,
    config_and_pool: &ConfigAndPool
) -> Result<(), SqlMiddlewareDbError> {
    // let config_and_pool = SqliteMiddlewarePoolConnection::new("sqlite::memory:").unwrap();
    let json2 = json1.clone();

    let json = json2;
    let pool = config_and_pool.pool.get().await.map_err(SqlMiddlewareDbError::from)?;
    let conn = MiddlewarePool::get_connection(pool).await.map_err(SqlMiddlewareDbError::from)?;

    let x = match &conn {
        MiddlewarePoolConnection::Sqlite(sconn) => {
            // let conn = conn.lock().unwrap();
            sconn.interact(move |xxx| {
                if cfg!(debug_assertions) {
                    #[allow(unused_variables)]
                    let pretty_json = serde_json::to_string_pretty(&json).unwrap();
                    // println!("{}", pretty_json);
                }

                let query_and_params_vec = QueryAndParams {
                    query: "SELECT * FROM event WHERE event_id = ?1 AND year = ?2;".to_string(),
                    params: vec![
                        RowValues::Int(json["event"].as_i64().unwrap()),
                        RowValues::Int(json["year"].as_i64().unwrap())
                    ],
                };
                let tx = xxx.transaction()?;
                {
                    let converted_params = sqlite_convert_params(&query_and_params_vec.params)?;
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
                                RowValues::Int(json["event"].as_i64().unwrap()),
                                RowValues::Int(json["year"].as_i64().unwrap()),
                                RowValues::Text(json["name"].to_string())
                            ],
                        };
                        let converted_params = sql_middleware::sqlite_convert_params_for_execute(
                            query_and_params_vec.params
                        )?;

                        let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                        stmt.execute(converted_params)?;

                        let query_and_params_vec = QueryAndParams {
                            query: "SELECT * FROM event WHERE event_id = ?1 AND year = ?2;".to_string(),
                            params: vec![
                                RowValues::Int(json["event"].as_i64().unwrap()),
                                RowValues::Int(json["year"].as_i64().unwrap())
                            ],
                        };
                        let converted_params = sql_middleware::sqlite_convert_params(
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
                        assert_eq!(result_set.results.len(), 1);

                        let data_to_fill = json["data_to_fill_if_event_and_year_missing"]
                            .as_array()
                            .unwrap();
                        for data in data_to_fill {
                            let bettors = data["bettors"].as_array().unwrap();
                            for bettor in bettors {
                                let query_and_params_vec = QueryAndParams {
                                    query: "INSERT INTO bettor (name) SELECT ?1 WHERE NOT EXISTS (SELECT 1 from bettor where name = ?1);".to_string(),
                                    params: vec![RowValues::Text(bettor.to_string())],
                                };
                                let converted_params =
                                    sql_middleware::sqlite_convert_params_for_execute(
                                        query_and_params_vec.params
                                    )?;

                                let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                                stmt.execute(converted_params)?;
                            }
                            let golfers = data["golfers"].as_array().unwrap();
                            for golfer in golfers {
                                let query_and_params_vec = QueryAndParams {
                                    query: "INSERT INTO golfer (name, espn_id) SELECT ?1, ?2 WHERE NOT EXISTS (SELECT 1 from golfer where espn_id = ?2);".to_string(),
                                    params: vec![
                                        RowValues::Text(golfer["name"].to_string()),
                                        RowValues::Int(golfer["espn_id"].as_i64().unwrap())
                                    ],
                                };
                                let converted_params =
                                    sql_middleware::sqlite_convert_params_for_execute(
                                        query_and_params_vec.params
                                    )?;

                                let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                                stmt.execute(converted_params)?;
                            }
                            let event_user_player = data["event_user_player"].as_array().unwrap();
                            for event_user_player in event_user_player {
                                let query_and_params_vec = QueryAndParams {
                                    query: "INSERT INTO event_user_player (event_id, user_id, golfer_id) 
                                    select (select event_id from event where espn_id = ?1)
                                    , (select user_id from bettor where name = ?2)
                                    , (select golfer_id from golfer where espn_id = ?3);".to_string(),
                                    params: vec![
                                        RowValues::Int(json["event"].as_i64().unwrap()),
                                        RowValues::Text(event_user_player["bettor"].to_string()),
                                        RowValues::Int(
                                            event_user_player["golfer_espn_id"].as_i64().unwrap()
                                        )
                                    ],
                                };
                                let converted_params =
                                    sql_middleware::sqlite_convert_params_for_execute(
                                        query_and_params_vec.params
                                    )?;

                                let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                                stmt.execute(converted_params)?;
                            }
                        }
                    } else {
                        let x = format!(
                            "Event {} and year {} already exist in the db. Skipping db prefill.",
                            json["event"].as_i64().unwrap(),
                            json["year"].as_i64().unwrap()
                        );
                        println!("{}", x.to_string());
                    }
                }
                tx.commit()?;
                Ok::<_, SqlMiddlewareDbError>(())
            }).await
            // .map_err(|e| format!("Error executing query: {:?}", e))
        }
        _ => unimplemented!(),
    };

    x?
}
