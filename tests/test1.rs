// // tests/integration_test.rs

// use actix_web::{test, App};
// use serde_json::Value;
// use sqlx::sqlite::SqlitePoolOptions;
// use std::collections::HashMap;
// use std::sync::Arc;
// use tokio::sync::RwLock;

// use rusty_golf::controller::score::scores; // Replace `your_crate` with your actual crate name
// use rusty_golf::db::db::{DatabaseType, Db, DbConfigAndPool};
// use rusty_golf::model::CacheMap;

// #[tokio::test]
// async fn test_scores_endpoint() {
//     // Initialize logging (optional, but useful for debugging)
//     let _ = env_logger::builder().is_test(true).try_init();

//     // Step 1: Set up an in-memory SQLite database
//     let sqlite_pool = SqlitePoolOptions::new()
//         .max_connections(5)
//         .connect(":memory:")
//         .await
//         .expect("Failed to create SQLite pool");

//     // Step 2: Create necessary tables
//     // Adjust the SQL schema according to your actual database schema
//     sqlx::query(
//         r#"
//         CREATE TABLE scores (
//             id INTEGER PRIMARY KEY AUTOINCREMENT,
//             event_id INTEGER NOT NULL,
//             year INTEGER NOT NULL,
//             team TEXT NOT NULL,
//             score INTEGER NOT NULL
//         );
//         "#,
//     )
//     .execute(&sqlite_pool)
//     .await
//     .expect("Failed to create scores table");

//     // Step 3: Insert test data
//     sqlx::query(
//         r#"
//         INSERT INTO scores (event_id, year, team, score) VALUES
//         (12345, 2024, 'Team A', 10),
//         (12345, 2024, 'Team B', 15),
//         (12345, 2024, 'Team C', 20);
//         "#,
//     )
//     .execute(&sqlite_pool)
//     .await
//     .expect("Failed to insert test data");

//     // Step 4: Set up DbConfigAndPool
//     let db_config_and_pool = DbConfigAndPool {
//         pool: your_crate::db::db::DbPool::Sqlite(sqlite_pool.clone()),
//     };

//     // Step 5: Initialize the cache
//     let cache_map: CacheMap = Arc::new(RwLock::new(HashMap::new()));

//     // Step 6: Initialize the Actix-web App with the `/scores` route
//     let app = test::init_service(
//         App::new()
//             .app_data(actix_web::web::Data::new(cache_map.clone()))
//             .app_data(actix_web::web::Data::new(db_config_and_pool.clone()))
//             .route("/scores", actix_web::web::get().to(scores)),
//     )
//     .await;

//     // Step 7: Define query parameters
//     let query_params = HashMap::from([
//         ("event", "12345".to_string()),
//         ("yr", "2024".to_string()),
//         ("cache", "false".to_string()),
//         ("json", "true".to_string()),
//     ]);

//     // Build the request URI with query parameters
//     let req = test::TestRequest::get()
//         .uri(&format!(
//             "/scores?event={}&yr={}&cache={}&json={}",
//             query_params["event"],
//             query_params["yr"],
//             query_params["cache"],
//             query_params["json"]
//         ))
//         .to_request();

//     // Step 8: Send the request and get the response
//     let resp = test::call_service(&app, req).await;

//     // Step 9: Assert the response status
//     assert!(resp.status().is_success(), "Response was not successful");

//     // Step 10: Parse the response body as JSON
//     let body: Value = test::read_body_json(resp).await;

//     // Step 11: Assert the JSON structure
//     assert!(body.is_object(), "Response is not a JSON object");
//     assert!(body.get("data").is_some(), "Response JSON does not contain 'data' field");
//     assert!(body.get("error").is_none(), "Response JSON contains 'error' field");

//     // Further assertions based on expected data
//     let data = body.get("data").unwrap();
//     // Assuming data is an array of scores
//     assert!(data.is_array(), "'data' field is not an array");

//     let scores_array = data.as_array().unwrap();
//     assert_eq!(scores_array.len(), 3, "Unexpected number of scores returned");

//     // Check individual score entries
//     for score in scores_array {
//         assert!(score.get("team").is_some(), "Score entry missing 'team'");
//         assert!(score.get("score").is_some(), "Score entry missing 'score'");
//     }
// }
