use rusty_golf::controller::db_prefill;
use crate::db_prefill::db_prefill;
// `, `use rusty_golf::controller::db_prefill::db_prefill;

// use rusty_golf::controller::score;

use sql_middleware::middleware::{
    AsyncDatabaseExecutor,
    ConfigAndPool as ConfigAndPool2,
    MiddlewarePool,
    QueryAndParams,
};

#[tokio::test]
async fn test_dbprefill() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (optional, but useful for debugging)
    // let _ = env_logger::builder().is_test(true).try_init();

    // let x = "file::memory:?cache=shared".to_string();
    let x = "zzz".to_string();
    let config_and_pool = ConfigAndPool2::new_sqlite(x).await.unwrap();

    let ddl = vec![
        include_str!("../src/admin/model/sql/schema/sqlite/00_event.sql"),
        // include_str!("../src/admin/model/sql/schema/sqlite/01_golfstatistic.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/02_golfer.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/03_bettor.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/04_event_user_player.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/05_eup_statistic.sql")
    ];

    let query_and_params = QueryAndParams {
        query: ddl.join("\n"),
        params: vec![],
    };

    let pool = config_and_pool.pool.get().await?;
    let mut conn = MiddlewarePool::get_connection(pool).await?;
    conn.execute_batch(&query_and_params.query).await?;

    // first verify that nothing is in these tables
    let query = "select * from event;";
    let res = conn.execute_select(&query, &vec![]).await?;
    assert_eq!(res.results.len(), 0);
    let query = "select * from golfer;";
    let res = conn.execute_select(&query, &vec![]).await?;
    assert_eq!(res.results.len(), 0);
    let query = "select * from bettor;";
    let res = conn.execute_select(&query, &vec![]).await?;
    assert_eq!(res.results.len(), 0);
    let query = "select * from event_user_player;";
    let res = conn.execute_select(&query, &vec![]).await?;
    assert_eq!(res.results.len(), 0);

    let json = serde_json::from_str(include_str!("test5_dbprefill.json"))?;
    db_prefill(&json, &config_and_pool).await?;

    // now verify that the tables have been populated
    let query = "select * from event;";
    let res = conn.execute_select(&query, &vec![]).await?;
    assert_eq!(res.results.len(), 1);

    let query = "select * from golfer;";
    let res = conn.execute_select(&query, &vec![]).await?;
    assert_eq!(res.results.len(), 22);
    let x = res.results.iter().find(|z| *z.get("espn_id").unwrap().as_int().unwrap() == 4375972);
    assert_eq!(x.is_some(), true);
    assert_eq!(x.unwrap().get("name").unwrap().as_text().unwrap(), "Ludvig Åberg");

    let query = "select * from bettor;";
    let res = conn.execute_select(&query, &vec![]).await?;
    assert_eq!(res.results.len(), 5);
    let x = res.results.iter().find(|z| z.get("name").unwrap().as_text().unwrap() == "Player5");
    assert_eq!(x.is_some(), true);

    let query = "select * from event_user_player;";
    let res = conn.execute_select(&query, &vec![]).await?;
    assert_eq!(res.results.len(), 15);
    let x = res.results
        .iter()
        .find(
            |z|
                z.get("bettor").unwrap().as_text().unwrap() == "Player3" &&
                *z.get("golfer_espn_id").unwrap().as_int().unwrap() == 9780
        );
    assert_eq!(x.is_some(), true);

    Ok(())
}
