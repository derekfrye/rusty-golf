use actix_web::{test, App};
use serde_json::Value;

use sqlx_middleware::convenience_items::{ create_tables3, MissingDbObjects};
use sqlx_middleware::middleware::CheckType;
// use sqlx_middleware::model::{CheckType, QueryAndParams};
// use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
use std::{collections::HashMap, vec};
use tokio::sync::RwLock;

// use rusty_golf::controller::score;
use rusty_golf::{controller::score::scores, model::CacheMap};
// use sqlx_middleware::db::{ConfigAndPool, Db, QueryState};
use sqlx_middleware::{
    middleware::{
        ConfigAndPool, MiddlewarePool,
        MiddlewarePoolConnection,
        QueryAndParams, //RowValues, QueryState
    },
    SqlMiddlewareDbError,
};

#[tokio::test]
async fn test_scores_endpoint() {
    // Initialize logging (optional, but useful for debugging)
    // let _ = env_logger::builder().is_test(true).try_init();

    // let mut cfg = deadpool_postgres::Config::new();
    let x = "file::memory:?cache=shared".to_string();

    let config_and_pool =
        ConfigAndPool::new_sqlite(x).await.unwrap();
    // let sql_db = Db::new(sqlite_configandpool.clone()).unwrap();

    let tables = vec![
        "event",
        // "golfstatistic",
        "golfer",
        "bettor",
        "event_user_player",
        "eup_statistic",
    ];
    let ddl = vec![
        include_str!("../src/admin/model/sql/schema/sqlite/00_event.sql"),
        // include_str!("../src/admin/model/sql/schema/sqlite/01_golfstatistic.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/02_golfer.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/03_bettor.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/04_event_user_player.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/05_eup_statistic.sql"),
    ];

    // fixme, the conv item function shouldnt require a 4-len str array, that's silly
    let mut table_ddl = vec![];
    for (i, table) in tables.iter().enumerate() {
        table_ddl.push((table, ddl[i], "", ""));
    }

    let mut missing_objs: Vec<MissingDbObjects> = vec![];
    for table in table_ddl.iter() {
        missing_objs.push(MissingDbObjects {
            missing_object: table.0.to_string(),
        });
    }

    let _create_result = create_tables3(
        &config_and_pool,
        missing_objs,
        CheckType::Table,
        &table_ddl
            .iter()
            .map(|(a, b, c, d)| (**a, *b, *c, *d))
            .collect::<Vec<_>>(),
    )
    .await
    .unwrap();

    // no need to check these things, if its going to err, it'll err

    // if create_result.db_last_exec_state == QueryState::QueryError {
    //     eprintln!("Error: {}", create_result.error_message.unwrap());
    // }
    // assert_eq!(
    //     create_result.db_last_exec_state,
    //     QueryState::QueryReturnedSuccessfully
    // );
    // assert_eq!(create_result.return_result, String::default());

    // // Step 1: Set up an in-memory SQLite database
    // let sqlite_pool = SqlitePoolOptions::new()
    //     .max_connections(5)
    //     .connect(":memory:")
    //     .await
    //     .expect("Failed to create SQLite pool");

    let setup_queries = include_str!("test1.sql");
    let query_and_params = QueryAndParams {
        query: setup_queries.to_string(),
        params: vec![],
        is_read_only: false,
    };

    let pool = config_and_pool.pool.get().await.unwrap();
        let conn = MiddlewarePool::get_connection(pool).await.unwrap();

        let _res = match &conn {
            MiddlewarePoolConnection::Sqlite(sconn) => {
                // let conn = conn.lock().unwrap();
                sconn
                    .interact(move |xxx| {
                        let converted_params = sqlx_middleware::sqlite_convert_params(
                            &query_and_params.params
                        )?;
                        let tx = xxx.transaction()?;

                        let result_set = {
                            let mut stmt = tx.prepare(&query_and_params.query)?;
                            let rs = sqlx_middleware::sqlite_build_result_set(
                                &mut stmt,
                                &converted_params
                            )?;
                            rs
                        };
                        Ok::<_, SqlMiddlewareDbError>(result_set)
                    }).await
                    .map_err(|e| format!("Error executing query: {:?}", e))
            }
            _ => {
                panic!("Only sqlite is supported ");
            }
            // Result::<(), String>::Ok(())
    }.unwrap();        

    // let res = sql_db
    //     .exec_general_query(vec![query_and_params], false)
    //     .await
    //     .unwrap();
    // assert_eq!(
    //     res.db_last_exec_state,
    //     QueryState::QueryReturnedSuccessfully
    // );

    // Step 5: Initialize the cache
    let cache_map: CacheMap = Arc::new(RwLock::new(HashMap::new()));

    // Step 6: Initialize the Actix-web App with the `/scores` route
    let app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(cache_map.clone()))
            .app_data(actix_web::web::Data::new(config_and_pool.clone()))
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
        _status => {
           

    // Step 9: Assert the response status
//     let resp_body = test::read_body(resp).await;
// assert!(
//     resp.status().is_success(),
//     "Response was not successful: {}",
//     String::from_utf8_lossy(&resp_body)
// );

    // Step 10: Parse the response body as JSON
    let body: Value = test::read_body_json(resp).await;
    println!("Failed with status: {}, body: {}", status, String::from_utf8_lossy(&body));

    // println!("{}", serde_json::to_string_pretty(&body).unwrap());

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
        let scoreboard_position = bettor.get("scoreboard_position").unwrap().as_i64().unwrap();

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
    }}}
}
