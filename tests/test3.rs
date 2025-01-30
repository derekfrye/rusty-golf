use rusty_golf::admin::model::admin_model::MissingDbObjects;
use serde_json::Value;
// use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
use std::{collections::HashMap, vec};
use tokio::sync::RwLock;

// use rusty_golf::controller::score;
use rusty_golf::{controller::score::get_data_for_scores_page, model::CacheMap};

use sqlx_middleware::middleware::ConfigAndPool as ConfigAndPool2;

#[tokio::test]
async fn test_get_data_for_scores_page() -> Result<(), Box<Box<dyn std::error::Error>>> {
    // Initialize logging (optional, but useful for debugging)
    // let _ = env_logger::builder().is_test(true).try_init();


    let x = "file::memory:?cache=shared".to_string();
    let config_and_pool = ConfigAndPool2::new_sqlite(x.clone()).await.unwrap();
    let mut cfg = deadpool_postgres::Config::new();
    cfg.dbname = Some(x);

    let sqlite_configandpool =
        ConfigAndPool2::new_sqlite(x).await;
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

    let _create_result = create_tables(
        &sql_db,
        missing_objs,
        CheckType::Table,
        &table_ddl
            .iter()
            .map(|(a, b, c, d)| (**a, *b, *c, *d))
            .collect::<Vec<_>>(),
    )
    .await
    .unwrap();

    let setup_queries = include_str!("test1.sql");
    let query_and_params = QueryAndParams {
        query: setup_queries.to_string(),
        params: vec![],
    };
    let _res = sql_db
        .exec_general_query(vec![query_and_params], false)
        .await
        .unwrap();

    let cache_map: CacheMap = Arc::new(RwLock::new(HashMap::new()));
    let x = match get_data_for_scores_page(401580351, 2024, &cache_map, false, &config_and_pool, 0).await {
        Ok(data) => data,
        Err(e) => return Err(Box::new(e)),
    };

    // load reference json
    let reference_result_str = include_str!("test3_espn_json_responses.json");
    let reference_result: Value = serde_json::from_str(reference_result_str).unwrap();

    let bryson_espn_entry = x
        .score_struct
        .iter()
        .find(|s| s.golfer_name == "Bryson DeChambeau")
        .unwrap();
    let bryson_reference_entry = reference_result
        .get("score_struct")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s.get("golfer_name").unwrap() == "Bryson DeChambeau")
        .unwrap();
    assert_eq!(
        bryson_espn_entry.detailed_statistics.total_score as i64,
        bryson_reference_entry
            .get("detailed_statistics")
            .unwrap()
            .get("total_score")
            .unwrap()
            .as_i64()
            .unwrap()
    );
    assert_eq!(
        bryson_espn_entry.eup_id,
        bryson_reference_entry
            .get("eup_id")
            .unwrap()
            .as_i64()
            .unwrap()
    );

    // let xx = serde_json::to_string_pretty(&x).unwrap();
    // println!("{}", xx); // test3_scoredata.json
    // let a = 1 + 1;

    let left = bryson_reference_entry
        .get("detailed_statistics")
        .unwrap()
        .get("line_scores")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|s| {
            s.get("hole").unwrap().as_i64().unwrap() == 13
                && s.get("round").unwrap().as_i64().unwrap() == 2
        })
        .unwrap()
        .get("score")
        .unwrap()
        .as_i64()
        .unwrap() as i32;
    println!("{}", left);
    let right = bryson_espn_entry
        .detailed_statistics
        .line_scores
        .iter()
        .find(|s| s.hole == 13 && s.round == 2)
        .unwrap()
        .score;
    println!("{}", right);

    assert_eq!(left, right);
    assert_eq!(left, 3); // line 6824 in test3_espn_json_responses.json

    Ok(())

    // let active_golfers = model::get_golfers_from_db(&sql_db, 401580351).await.unwrap().return_result;

    // let scores = fetch_scores_from_espn(active_golfers.clone(), 2024, 401580351).await.unwrap();

    // Step 5: Initialize the cache
    // let cache_map: CacheMap = Arc::new(RwLock::new(HashMap::new()));

    // // Step 6: Initialize the Actix-web App with the `/scores` route
    // let app = test::init_service(
    //     App::new()
    //         .app_data(actix_web::web::Data::new(cache_map.clone()))
    //         .app_data(actix_web::web::Data::new(sqlite_configandpool.clone()))
    //         .route("/scores", actix_web::web::get().to(scores)),
    // )
    // .await;

    // // Step 7: Define query parameters
    // let query_params = HashMap::from([
    //     ("event", "401580351".to_string()),
    //     ("yr", "2024".to_string()),
    //     ("cache", "false".to_string()),
    //     ("json", "true".to_string()),
    // ]);

    // // Build the request URI with query parameters
    // let req = test::TestRequest::get()
    //     .uri(&format!(
    //         "/scores?event={}&yr={}&cache={}&json={}",
    //         query_params["event"], query_params["yr"], query_params["cache"], query_params["json"]
    //     ))
    //     .to_request();

    // // Step 8: Send the request and get the response
    // let resp = test::call_service(&app, req).await;

    // // Step 9: Assert the response status
    // assert!(resp.status().is_success(), "Response was not successful");

    // // Step 10: Parse the response body as JSON
    // let body: Value = test::read_body_json(resp).await;

    // // println!("{}", serde_json::to_string_pretty(&body).unwrap());

    // // Step 11: Assert the JSON structure
    // assert!(body.is_object(), "Response is not a JSON object");
    // assert!(
    //     body.get("bettor_struct").is_some(),
    //     "Response JSON does not contain 'bettor_struct' field"
    // );

    // let bettor_struct = body.get("bettor_struct").unwrap();
    // assert!(
    //     bettor_struct.is_array(),
    //     "'bettor_struct' field is not an array"
    // );

    // let bettor_struct_array = bettor_struct.as_array().unwrap();
    // assert_eq!(
    //     bettor_struct_array.len(),
    //     5,
    //     "Unexpected number of bettors returned"
    // );

    // // load reference json
    // let reference_result_str = include_str!("test1_expected_output.json");
    // let reference_result: Value = serde_json::from_str(reference_result_str).unwrap();

    // // Check individual score entries
    // for bettor in bettor_struct_array {
    //     assert!(
    //         bettor.get("bettor_name").is_some(),
    //         "Score entry missing 'bettor_name'"
    //     );
    //     assert!(
    //         bettor.get("total_score").is_some(),
    //         "Score entry missing 'total_score'"
    //     );
    //     assert!(
    //         bettor.get("scoreboard_position").is_some(),
    //         "Score entry missing 'scoreboard_position'"
    //     );

    //     let bettor_name = bettor.get("bettor_name").unwrap().as_str().unwrap();
    //     let total_score = bettor.get("total_score").unwrap().as_i64().unwrap();
    //     let scoreboard_position = bettor.get("scoreboard_position").unwrap().as_i64().unwrap();

    //     // Check if the bettor name is in the reference JSON
    //     let reference_array = reference_result
    //         .get("bettor_struct")
    //         .unwrap()
    //         .as_array()
    //         .unwrap();
    //     let reference_bettor = reference_array
    //         .iter()
    //         .find(|s| s.get("bettor_name").unwrap().as_str().unwrap() == bettor_name)
    //         .unwrap();

    //     // Check if the total score matches
    //     assert_eq!(
    //         total_score,
    //         reference_bettor
    //             .get("total_score")
    //             .unwrap()
    //             .as_i64()
    //             .unwrap(),
    //         "Total score mismatch for bettor '{}', expected {}, got {}",
    //         bettor_name,
    //         total_score,
    //         reference_bettor
    //             .get("total_score")
    //             .unwrap()
    //             .as_i64()
    //             .unwrap()
    //     );

    //     // Check if the scoreboard position matches
    //     assert_eq!(
    //         scoreboard_position,
    //         reference_bettor
    //             .get("scoreboard_position")
    //             .unwrap()
    //             .as_i64()
    //             .unwrap(),
    //         "Scoreboard position mismatch for bettor '{}', expected {}, got {}",
    //         bettor_name,
    //         scoreboard_position,
    //         reference_bettor
    //             .get("scoreboard_position")
    //             .unwrap()
    //             .as_i64()
    //             .unwrap()
    //     );

    //     // Check if the scoreboard position name matches
    //     assert_eq!(
    //         bettor
    //             .get("scoreboard_position_name")
    //             .unwrap()
    //             .as_str()
    //             .unwrap(),
    //         reference_bettor
    //             .get("scoreboard_position_name")
    //             .unwrap()
    //             .as_str()
    //             .unwrap(),
    //         "Scoreboard position name mismatch for bettor '{}', expected '{}', got '{}'",
    //         bettor_name,
    //         bettor
    //             .get("scoreboard_position_name")
    //             .unwrap()
    //             .as_str()
    //             .unwrap(),
    //         reference_bettor
    //             .get("scoreboard_position_name")
    //             .unwrap()
    //             .as_str()
    //             .unwrap()
    //     );
    // }
}
