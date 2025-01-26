use serde_json::Value;
// use sqlx_middleware::db::{ConfigAndPool, DatabaseType, Db, QueryState};
use sqlx_middleware::{
    db_model::{
        ConfigAndPool, MiddlewarePool,
        MiddlewarePoolConnection,
        QueryAndParams, RowValues,
    },
    SqlMiddlewareDbError,
};
use tokio::runtime::Runtime;

/// format we have is this:
/// [{ "event": <int>, "year": <int>, "name": "value",  "data_to_fill_if_event_and_year_missing": [
/// { "bettors": [{"PlayerName", "PlayerName2", "PlayerName3"...}]
/// , "golfers": [{"name": "Firstname Lastname", "espn_id": <int>}, {"name": "Firstname Lastname", "espn_id": <int>}, ...]
/// , "event_user_player": [{"bettor": "PlayerName", "golfer_espn_id": <int>}, {"bettor": "PlayerName", "golfer_espn_id": <int>}, ...]
/// }]}]
pub fn db_prefill(json1: &Value, config_and_pool: &ConfigAndPool) -> Result<(), String> {
    // let config_and_pool = SqliteMiddlewarePoolConnection::new("sqlite::memory:").unwrap();
    let json2 = json1.clone();
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let json = json2;
        let pool = config_and_pool.pool.get().await.map_err(|e| e.to_string())?;
        let conn = MiddlewarePool::get_connection(pool).await.map_err(|e| e.to_string())?;

        let _res = (match &conn {
            MiddlewarePoolConnection::Sqlite(sconn) => {
                // let conn = conn.lock().unwrap();
                sconn
                    .interact(move |xxx| {
                        let query_and_params_vec = QueryAndParams {
                            query: "SELECT * FROM event WHERE event_id = ?1 AND year = ?2;".to_string(),
                            params: vec![
                                RowValues::Int(json["event"].as_i64().unwrap()),
                                RowValues::Int(json["year"].as_i64().unwrap())
                            ],
                            is_read_only: true,
                        };
                        let tx = xxx.transaction()?;

                        let converted_params = sqlx_middleware::sqlite_convert_params(
                            &query_and_params_vec.params
                        )?;
                        let result_set = {
                            let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                            let rs = sqlx_middleware::sqlite_build_result_set(
                                &mut stmt,
                                &converted_params
                            )?;
                            rs
                        };
                        if result_set.results.len() > 0 {
                            let query_and_params_vec = QueryAndParams {
                                query: "INSERT INTO event (name, espn_id, year) VALUES(?1, ?2, ?3);".to_string(),
                                params: vec![
                                    RowValues::Int(json["event"].as_i64().unwrap()),
                                    RowValues::Int(json["year"].as_i64().unwrap())
                                ],
                                is_read_only: false,
                            };
                            let converted_params = sqlx_middleware::sqlite_convert_params(
                                &query_and_params_vec.params
                            )?;
                            let _result_set = {
                                let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                                let rs = sqlx_middleware::sqlite_build_result_set(
                                    &mut stmt,
                                    &converted_params
                                )?;
                                rs
                            };

                            let query_and_params_vec = QueryAndParams {
                                query: "INSERT INTO golfer (name, espn_id) SELECT ?1, ?2 
                                WHERE NOT EXISTS (SELECT 1 from golfer where espn_id = ?2);".to_string(),
                                params: vec![
                                    RowValues::Int(json["event"].as_i64().unwrap()),
                                    RowValues::Int(json["year"].as_i64().unwrap())
                                ],
                                is_read_only: false,
                            };
                            let converted_params = sqlx_middleware::sqlite_convert_params(
                                &query_and_params_vec.params
                            )?;
                            let _result_set = {
                                let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                                let rs = sqlx_middleware::sqlite_build_result_set(
                                    &mut stmt,
                                    &converted_params
                                )?;
                                rs
                            };

                            let data_to_fill = json["data_to_fill_if_event_and_year_missing"]
                                .as_array()
                                .unwrap();
                            for data in data_to_fill {
                                let bettors = data["bettors"].as_array().unwrap();
                                for bettor in bettors {
                                    let query_and_params_vec = QueryAndParams {
                                        query: "INSERT INTO bettor (name) SELECT ?1 WHERE NOT EXISTS (SELECT 1 from bettor where name = ?1);".to_string(),
                                        params: vec![RowValues::Text(bettor.to_string())],
                                        is_read_only: false,
                                    };
                                    let converted_params = sqlx_middleware::sqlite_convert_params(
                                        &query_and_params_vec.params
                                    )?;
                                    let _result_set = {
                                        let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                                        let rs = sqlx_middleware::sqlite_build_result_set(
                                            &mut stmt,
                                            &converted_params
                                        )?;
                                        rs
                                    };
                                }
                                let golfers = data["golfers"].as_array().unwrap();
                                for golfer in golfers {
                                    let query_and_params_vec = QueryAndParams {
                                        query: "INSERT INTO golfer (name, espn_id) SELECT ?1, ?2 WHERE NOT EXISTS (SELECT 1 from golfer where espn_id = ?2);".to_string(),
                                        params: vec![
                                            RowValues::Text(golfer["name"].to_string()),
                                            RowValues::Int(golfer["espn_id"].as_i64().unwrap())
                                        ],
                                        is_read_only: false,
                                    };
                                    let converted_params = sqlx_middleware::sqlite_convert_params(
                                        &query_and_params_vec.params
                                    )?;
                                    let _result_set = {
                                        let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                                        let rs = sqlx_middleware::sqlite_build_result_set(
                                            &mut stmt,
                                            &converted_params
                                        )?;
                                        rs
                                    };
                                }
                                let event_user_player = data["event_user_player"]
                                    .as_array()
                                    .unwrap();
                                for event_user_player in event_user_player {
                                    let query_and_params_vec = QueryAndParams {
                                        query: "INSERT INTO event_user_player (event_id, user_id, golfer_id) 
                                    select (select event_id from event where espn_id = ?1)
                                    , (select user_id from bettor where name = ?2)
                                    , (select golfer_id from golfer where espn_id = ?3)
                                    
                                    ".to_string(),
                                        params: vec![
                                            RowValues::Int(json["event"].as_i64().unwrap()),
                                            RowValues::Text(
                                                event_user_player["bettor"].to_string()
                                            ),
                                            RowValues::Int(
                                                event_user_player["golfer_espn_id"]
                                                    .as_i64()
                                                    .unwrap()
                                            )
                                        ],
                                        is_read_only: false,
                                    };
                                    let converted_params = sqlx_middleware::sqlite_convert_params(
                                        &query_and_params_vec.params
                                    )?;
                                    let _result_set = {
                                        let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                                        let rs = sqlx_middleware::sqlite_build_result_set(
                                            &mut stmt,
                                            &converted_params
                                        )?;
                                        rs
                                    };
                                }
                            }
                        } else {
                            println!(
                                "Event and year already exist in the db. Skipping db prefill."
                            );
                        }
                        tx.commit()?;
                        Ok::<_, SqlMiddlewareDbError>(result_set)
                    }).await
                    .map_err(|e| format!("Error executing query: {:?}", e))
            }
            _ => {
                return Err("Only sqlite is supported for db prefill".to_string());
            }
        })?;

        Result::<(), String>::Ok(())
    })?;
    Ok(())
}
