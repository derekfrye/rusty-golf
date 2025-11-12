use actix_web::{App, test};
use rusty_golf::args::CleanArgs;
use serde_json::Value;

// use sqlx_middleware::convenience_items::{create_tables3, MissingDbObjects};
// use sqlx_middleware::model::{CheckType, QueryAndParams};
// use sqlx::sqlite::SqlitePoolOptions;
use std::{collections::HashMap, vec};

// use rusty_golf::controller::score;
use rusty_golf::controller::score::scores;
// use sqlx_middleware::db::{ConfigAndPool, Db, QueryState};
use sql_middleware::{
    SqlMiddlewareDbError,
    middleware::{
        ConfigAndPool,
        MiddlewarePool,
        MiddlewarePoolConnection,
        QueryAndParams, //RowValues, QueryState
    },
};

#[test]
async fn test1_scores_endpoint() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (optional, but useful for debugging)
    // let _ = env_logger::builder().is_test(true).try_init();

    // let mut cfg = deadpool_postgres::Config::new();
    let x = "file::memory:?cache=shared".to_string();
    // let x = "xxx".to_string();
    let args = CleanArgs {
        db_type: sql_middleware::middleware::DatabaseType::Sqlite,
        db_name: x.clone(),
        db_host: None,
        db_port: None,
        db_user: None,
        db_password: None,
        db_startup_script: None,
        combined_sql_script: "".to_string(),
        db_populate_json: None,
    };

    let config_and_pool = ConfigAndPool::new_sqlite(x).await.unwrap();
    // let sql_db = Db::new(sqlite_configandpool.clone()).unwrap();

    let setup_queries = include_str!("../src/sql/schema/sqlite/00_table_drop.sql");
    let query_and_params = QueryAndParams {
        query: setup_queries.to_string(),
        params: vec![],
    };

    let pool = config_and_pool.pool.get().await.unwrap();
    let conn = MiddlewarePool::get_connection(pool).await.unwrap();

    let res: Result<_, SqlMiddlewareDbError> = match &conn {
        MiddlewarePoolConnection::Sqlite(sconn) => {
            sconn
                .with_connection(move |conn| {
                    let tx = conn.transaction()?;
                    tx.execute_batch(&query_and_params.query)?;
                    tx.commit()?;
                    Ok::<_, SqlMiddlewareDbError>(())
                })
                .await
        }
        _ => panic!("Only sqlite is supported "),
    };

    assert!(res.is_ok(), "Error executing query: {res:?}");

    let ddl = [
        include_str!("../src/sql/schema/sqlite/00_event.sql"),
        include_str!("../src/sql/schema/sqlite/02_golfer.sql"),
        include_str!("../src/sql/schema/sqlite/03_bettor.sql"),
        include_str!("../src/sql/schema/sqlite/04_event_user_player.sql"),
        include_str!("../src/sql/schema/sqlite/05_eup_statistic.sql"),
    ];

    let query_and_params = QueryAndParams {
        query: ddl.join("\n"),
        params: vec![],
    };

    match conn {
        MiddlewarePoolConnection::Postgres(mut xx) => {
            let tx = xx.transaction().await?;

            tx.batch_execute(&query_and_params.query).await?;
            tx.commit().await?;
            Ok::<_, SqlMiddlewareDbError>(())
        }
        MiddlewarePoolConnection::Sqlite(sqlite_conn) => {
            sqlite_conn
                .with_connection(move |conn| {
                    let tx = conn.transaction()?;
                    tx.execute_batch(&query_and_params.query)?;
                    tx.commit()?;
                    Ok::<_, SqlMiddlewareDbError>(())
                })
                .await?;
            Ok::<_, SqlMiddlewareDbError>(())
        } // MiddlewarePoolConnection::Mssql(_) => todo!()
    }?;

    let setup_queries = include_str!("test1.sql");
    let query_and_params = QueryAndParams {
        query: setup_queries.to_string(),
        params: vec![],
    };

    let pool = config_and_pool.pool.get().await.unwrap();
    let conn = MiddlewarePool::get_connection(pool).await.unwrap();

    let res: Result<_, SqlMiddlewareDbError> = match &conn {
        MiddlewarePoolConnection::Sqlite(sconn) => {
            sconn
                .with_connection(move |conn| {
                    let tx = conn.transaction()?;
                    tx.execute_batch(&query_and_params.query)?;
                    tx.commit()?;
                    Ok::<_, SqlMiddlewareDbError>(())
                })
                .await
        }
        _ => panic!("Only sqlite is supported "),
    };

    assert!(res.is_ok(), "Error executing query: {res:?}");

    // let res = sql_db
    //     .exec_general_query(vec![query_and_params], false)
    //     .await
    //     .unwrap();
    // assert_eq!(
    //     res.db_last_exec_state,
    //     QueryState::QueryReturnedSuccessfully
    // );

    // Step 6: Initialize the Actix-web App with the `/scores` route
    let app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(config_and_pool.clone()))
            .app_data(actix_web::web::Data::new(args.clone()))
            .route("/scores", actix_web::web::get().to(scores)),
    )
    .await;

    // Step 7: Define query parameters
    let query_params = HashMap::from([
        ("event", "401580351".to_string()),
        ("yr", "2024".to_string()),
        ("cache", "false".to_string()),
        ("json", "true".to_string()),
    ]);

    // Build the request URI with query parameters
    let req = test::TestRequest::get()
        .uri(&format!(
            "/scores?event={}&yr={}&cache={}&json={}",
            query_params["event"], query_params["yr"], query_params["cache"], query_params["json"]
        ))
        .to_request();

    // Step 8: Send the request and get the response
    let resp = test::call_service(&app, req).await;
    match resp.status() {
        actix_web::http::StatusCode::OK => {
            println!("Success!");
        }
        status => {
            // Step 9: Assert the response status
            //     let resp_body = test::read_body(resp).await;
            // assert!(
            //     resp.status().is_success(),
            //     "Response was not successful: {}",
            //     String::from_utf8_lossy(&resp_body)
            // );

            if status != actix_web::http::StatusCode::OK {
                println!("Failed with status: {status}");
            }

            // Step 10: Parse the response body as JSON
            let body: Value = test::read_body_json(resp).await;
            // println!("Failed with status: {}, body: {}", status, String::from_utf8_lossy(&body));

            // println!("{}", serde_json::to_string_pretty(&body).unwrap());
            let z = serde_json::to_string_pretty(&body).unwrap();
            if cfg!(debug_assertions) {
                println!("{z}");
            }

            // Step 11: Assert the JSON structure
            assert!(body.is_object(), "Response is not a JSON object");
            assert!(
                body.get("bettor_struct").is_some(),
                "Response JSON does not contain 'bettor_struct' field"
            );

            let bettor_struct = body.get("bettor_struct").unwrap();
            assert!(
                bettor_struct.is_array(),
                "'bettor_struct' field is not an array"
            );

            let bettor_struct_array = bettor_struct.as_array().unwrap();
            assert_eq!(
                bettor_struct_array.len(),
                5,
                "Unexpected number of bettors returned"
            );

            // load reference json
            let reference_result_str = include_str!("test1_expected_output.json");
            let reference_result: Value = serde_json::from_str(reference_result_str).unwrap();

            // Check individual score entries
            for bettor in bettor_struct_array {
                assert!(
                    bettor.get("bettor_name").is_some(),
                    "Score entry missing 'bettor_name'"
                );
                assert!(
                    bettor.get("total_score").is_some(),
                    "Score entry missing 'total_score'"
                );
                assert!(
                    bettor.get("scoreboard_position").is_some(),
                    "Score entry missing 'scoreboard_position'"
                );

                let bettor_name = bettor.get("bettor_name").unwrap().as_str().unwrap();
                let total_score = bettor.get("total_score").unwrap().as_i64().unwrap();
                let scoreboard_position =
                    bettor.get("scoreboard_position").unwrap().as_i64().unwrap();

                // Check if the bettor name is in the reference JSON
                let reference_array = reference_result
                    .get("bettor_struct")
                    .unwrap()
                    .as_array()
                    .unwrap();
                let reference_bettor = reference_array
                    .iter()
                    .find(|s| s.get("bettor_name").unwrap().as_str().unwrap() == bettor_name)
                    .unwrap();

                // Check if the total score matches
                assert_eq!(
                    total_score,
                    reference_bettor
                        .get("total_score")
                        .unwrap()
                        .as_i64()
                        .unwrap(),
                    "Total score mismatch for bettor '{}', expected {}, got {}",
                    bettor_name,
                    total_score,
                    reference_bettor
                        .get("total_score")
                        .unwrap()
                        .as_i64()
                        .unwrap()
                );

                // Check if the scoreboard position matches
                assert_eq!(
                    scoreboard_position,
                    reference_bettor
                        .get("scoreboard_position")
                        .unwrap()
                        .as_i64()
                        .unwrap(),
                    "Scoreboard position mismatch for bettor '{}', expected {}, got {}",
                    bettor_name,
                    scoreboard_position,
                    reference_bettor
                        .get("scoreboard_position")
                        .unwrap()
                        .as_i64()
                        .unwrap()
                );

                // Check if the scoreboard position name matches
                assert_eq!(
                    bettor
                        .get("scoreboard_position_name")
                        .unwrap()
                        .as_str()
                        .unwrap(),
                    reference_bettor
                        .get("scoreboard_position_name")
                        .unwrap()
                        .as_str()
                        .unwrap(),
                    "Scoreboard position name mismatch for bettor '{}', expected '{}', got '{}'",
                    bettor_name,
                    bettor
                        .get("scoreboard_position_name")
                        .unwrap()
                        .as_str()
                        .unwrap(),
                    reference_bettor
                        .get("scoreboard_position_name")
                        .unwrap()
                        .as_str()
                        .unwrap()
                );
            }
        }
    }

    Ok(())
}
