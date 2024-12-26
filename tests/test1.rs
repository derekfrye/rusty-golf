use actix_web::{test, App};
use serde_json::Value;
// use sqlx::query;
use sqlx_middleware::db::convenience_items::{create_tables, MissingDbObjects};
use sqlx_middleware::model::{CheckType, QueryAndParams, RowValues};
// use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
use std::{collections::HashMap, vec};
use tokio::sync::RwLock;

// use rusty_golf::controller::score;
use rusty_golf::{controller::score::scores, model::CacheMap};
use sqlx_middleware::db::db::{DatabaseSetupState, Db, DbConfigAndPool};

#[tokio::test]
async fn test_scores_endpoint() {
    // Initialize logging (optional, but useful for debugging)
    // let _ = env_logger::builder().is_test(true).try_init();

    let mut cfg = deadpool_postgres::Config::new();
    cfg.dbname = Some(":memory:".to_string());

    let sqlite_configandpool =
        DbConfigAndPool::new(cfg, sqlx_middleware::db::db::DatabaseType::Sqlite).await;
    let sql_db = Db::new(sqlite_configandpool.clone()).unwrap();

    let tables = vec![
        "event",
        "golfstatistic",
        "player",
        "golfuser",
        "event_user_player",
        "eup_statistic",
    ];
    let ddl = vec![
        include_str!("../src/admin/model/sql/schema/sqlite/00_event.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/01_golfstatistic.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/02_player.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/03_golfuser.sql"),
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

    let create_result = create_tables(
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

    if create_result.db_last_exec_state == DatabaseSetupState::QueryError {
        eprintln!("Error: {}", create_result.error_message.unwrap());
    }
    assert_eq!(
        create_result.db_last_exec_state,
        DatabaseSetupState::QueryReturnedSuccessfully
    );
    assert_eq!(create_result.return_result, String::default());

    let setup_queries = include_str!("test1_setup.sql");
    let query_and_params = QueryAndParams {
        query: setup_queries.to_string(),
        params: vec![],
    };
    let res = sql_db.exec_general_query(vec![query_and_params], false).await.unwrap();

    assert_eq!(res.db_last_exec_state, DatabaseSetupState::QueryReturnedSuccessfully);

    // now ck event_user_player has three entries for player1
    // lets use a param
    let qry = "SELECT count(*) as cnt FROM event_user_player WHERE player_id = ?;";
    // let params = vec![1];
    let param = "player1";
    let query_and_params = QueryAndParams {
        query: qry.to_string(),
        params: vec![RowValues::Text(param.to_string())],
    };
    let res = sql_db.exec_general_query(vec![query_and_params], true).await.unwrap();
    assert_eq!(res.db_last_exec_state, DatabaseSetupState::QueryReturnedSuccessfully);

    let count= res.return_result[0].results[0].get("cnt").unwrap().as_int().unwrap();
    // let cnt = count.
    assert_eq!(*count, 3);

    // let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();

    // let missing_tables = match result {
    //     Ok(r) => {
    //         dbresult.db_last_exec_state = r.db_last_exec_state;
    //         dbresult.error_message = r.error_message;
    //         r.return_result[0].results.clone()
    //     }
    //     Err(e) => {
    //         let emessage = format!("Failed in {}, {}: {}", std::file!(), std::line!(), e);
    //         let mut dbresult: DatabaseResult<String> = DatabaseResult::<String>::default();
    //         dbresult.error_message = Some(emessage);
    //         vec![]
    //     }
    // };

    // let zz: Vec<_> = missing_tables
    //     .iter()
    //     .filter_map(|row| {
    //         let exists_index = row.column_names.iter().position(|col| col == "eventname")?;

    //         match &row.rows[exists_index] {
    //             RowValues::Text(value) => Some(value),

    //             _ => None,
    //         }
    //     })
    //     .collect();
    // if !zz.is_empty() {
    //     dbresult.return_result = zz[0].to_string();
    // }

    // // Step 1: Set up an in-memory SQLite database
    // let sqlite_pool = SqlitePoolOptions::new()
    //     .max_connections(5)
    //     .connect(":memory:")
    //     .await
    //     .expect("Failed to create SQLite pool");

    // Step 5: Initialize the cache
    let cache_map: CacheMap = Arc::new(RwLock::new(HashMap::new()));

    // Step 6: Initialize the Actix-web App with the `/scores` route
    let app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(cache_map.clone()))
            .app_data(actix_web::web::Data::new(sqlite_configandpool.clone()))
            .route("/scores", actix_web::web::get().to(scores)),
    )
    .await;

    // Step 7: Define query parameters
    let query_params = HashMap::from([
        ("event", "12345".to_string()),
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

    // Step 9: Assert the response status
    assert!(resp.status().is_success(), "Response was not successful");

    // Step 10: Parse the response body as JSON
    let body: Value = test::read_body_json(resp).await;

    // Step 11: Assert the JSON structure
    assert!(body.is_object(), "Response is not a JSON object");
    assert!(
        body.get("data").is_some(),
        "Response JSON does not contain 'data' field"
    );
    assert!(
        body.get("error").is_none(),
        "Response JSON contains 'error' field"
    );

    // Further assertions based on expected data
    let data = body.get("data").unwrap();
    // Assuming data is an array of scores
    assert!(data.is_array(), "'data' field is not an array");

    let scores_array = data.as_array().unwrap();
    assert_eq!(
        scores_array.len(),
        3,
        "Unexpected number of scores returned"
    );

    // Check individual score entries
    for score in scores_array {
        assert!(score.get("team").is_some(), "Score entry missing 'team'");
        assert!(score.get("score").is_some(), "Score entry missing 'score'");
    }
}
