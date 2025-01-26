use serde_json::Value;
// use sqlx_middleware::db::{ConfigAndPool, DatabaseType, Db, QueryState};
use sqlx_middleware::{db_model::{ConfigAndPool, MiddlewarePool, MiddlewarePoolConnection::{
    self, Sqlite as SqliteMiddlewarePoolConnection,
}, QueryAndParams, RowValues}, SqlMiddlewareDbError};
use tokio::runtime::Runtime;

/// format we have is this:
/// [{ "event": <int>, "year": <int>, "data_to_fill_if_event_and_year_missing": [
/// { "bettors": [{"PlayerName", "PlayerName2", "PlayerName3"...}]
/// , "golfers": [{"name": "Firstname Lastname", "espn_id": <int>}, {"name": "Firstname Lastname", "espn_id": <int>}, ...]
/// , "event_user_player": [{"bettor": "PlayerName", "golfer_espn_id": <int>}, {"bettor": "PlayerName", "golfer_espn_id": <int>}, ...]
/// }]}]
pub fn db_prefill(json: &Value, config_and_pool: &ConfigAndPool) -> Result<(), String> {
    // let config_and_pool = SqliteMiddlewarePoolConnection::new("sqlite::memory:").unwrap();
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let pool = config_and_pool.pool.get().await.map_err(|e| e.to_string())?;
        let conn = MiddlewarePool::get_connection(pool).await.map_err(|e| e.to_string())?;

        let res = match &conn {
            MiddlewarePoolConnection::Sqlite(sconn) => {
                // let conn = conn.lock().unwrap();
                sconn.interact(move |xxx| {
                    let query_and_params_vec = QueryAndParams {
                        query: "SELECT * FROM event WHERE event_id = ?1 AND year = ?2".to_string(),
                        params: vec![ RowValues::Int( json["event"].as_i64()?), RowValues::Int( json["year"].as_i64()? ), ],
                        is_read_only: true,
                    };
                let mut tx = xxx.transaction()?;

                let converted_params = sqlx_middleware::sqlite_convert_params(&query_and_params_vec.params)?;
                let result_set={let mut stmt = tx.prepare(&query_and_params_vec.query)?;
                    let rs = sqlx_middleware::sqlite_build_result_set(&mut stmt, &converted_params)?;
                    rs};
                
                 = stmt.query(converted_params.iter())?;
                    let do_year_and_event_exist = tx.query_row("SELECT * FROM events WHERE event = $1 AND year = $2", &[&json["event"], &json["year"]])?;
                    if do_year_and_event_exist.is_empty() {
                        tx.execute("INSERT INTO events (event, year) VALUES ($1, $2)", &[&json["event"], &json["year"]])?;
                    

                        for data in json["data_to_fill_if_event_and_year_missing"].as_array().unwrap() {
                            for bettor in data["bettors"].as_array().unwrap() {
                                let bettor = bettor.as_str().unwrap();
                                tx.execute("INSERT INTO bettors (bettor) VALUES ($1)", &[&bettor])?;
                            }
                            for golfer in data["golfers"].as_array().unwrap() {
                                let name = golfer["name"].as_str().unwrap();
                                let espn_id = golfer["espn_id"].as_i64().unwrap();
                                tx.execute(
                                    "INSERT INTO golfers (name, espn_id) VALUES ($1, $2)",
                                    &[&name, &espn_id],
                                )?;
                            }
                            for event_user_player in data["event_user_player"].as_array().unwrap() {
                                let bettor = event_user_player["bettor"].as_str().unwrap();
                                let golfer_espn_id = event_user_player["golfer_espn_id"].as_i64().unwrap();
                                tx.execute(
                                    "INSERT INTO event_user_player (bettor, golfer_espn_id) VALUES ($1, $2)",
                                    &[&bettor, &golfer_espn_id],
                                )?;
                            }
                        }
                }
                    else {
                        println!("Event and year already exist in the db. Skipping db prefill.");
                    }
                tx.commit()?;
                Ok::<_, SqlMiddlewareDbError>(result_set)
            }).await
            .map_err(|e| format!("Error executing query: {:?}", e))
            }
            _ => {
                return Err("Only sqlite is supported for db prefill".to_string());
            }
        }?;

        Result::<(), String>::Ok(())
    })?;
    Ok(())
}