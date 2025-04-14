use crate::db_prefill::db_prefill;
use rusty_golf::controller::db_prefill;
// `, `use rusty_golf::controller::db_prefill::db_prefill;

// use rusty_golf::controller::score;

use sql_middleware::middleware::{
    AsyncDatabaseExecutor, ConfigAndPool as ConfigAndPool2, DatabaseType, MiddlewarePool,
    QueryAndParams, RowValues,
};

#[tokio::test]
async fn test_dbprefill() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (optional, but useful for debugging)
    // let _ = env_logger::builder().is_test(true).try_init();

    let x = "file::memory:?cache=shared".to_string();
    // let x = "zzz".to_string();
    let config_and_pool = ConfigAndPool2::new_sqlite(x).await.unwrap();

    let ddl = [
        include_str!("../src/admin/model/sql/schema/sqlite/00_event.sql"),
        // include_str!("../src/admin/model/sql/schema/sqlite/01_golfstatistic.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/02_golfer.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/03_bettor.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/04_event_user_player.sql"),
        include_str!("../src/admin/model/sql/schema/sqlite/05_eup_statistic.sql"),
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
    let res = conn.execute_select(query, &[]).await?;
    assert_eq!(res.results.len(), 0);
    let query = "select * from golfer;";
    let res = conn.execute_select(query, &[]).await?;
    assert_eq!(res.results.len(), 0);
    let query = "select * from bettor;";
    let res = conn.execute_select(query, &[]).await?;
    assert_eq!(res.results.len(), 0);
    let query = "select * from event_user_player;";
    let res = conn.execute_select(query, &[]).await?;
    assert_eq!(res.results.len(), 0);

    let json = serde_json::from_str(include_str!("test5_dbprefill.json"))?;
    db_prefill(&json, &config_and_pool, DatabaseType::Sqlite).await?;

    // now verify that the tables have been populated
    let query = "select * from event ;";
    let res = conn.execute_select(query, &[]).await?;
    assert_eq!(res.results.len(), 4);

    let query = "select * from golfer;";
    let res = conn.execute_select(query, &[]).await?;
    // x unq entries
    assert_eq!(res.results.len(), 23);
    let x = res
        .results
        .iter()
        .find(|z| *z.get("espn_id").unwrap().as_int().unwrap() == 4375972);
    assert!(x.is_some());
    assert_eq!(
        x.unwrap().get("name").unwrap().as_text().unwrap(),
        "Ludvig Åberg"
    );

    let query = "select * from bettor;";
    let res = conn.execute_select(query, &[]).await?;
    assert_eq!(res.results.len(), 5);
    let x = res
        .results
        .iter()
        .find(|z| z.get("name").unwrap().as_text().unwrap() == "Player5");
    assert!(x.is_some());

    let mut query = "select b.name as bettor, g.espn_id as golfer_espn_id ".to_string();
    query.push_str("from event_user_player as eup ");
    query.push_str("join bettor as b on b.user_id = eup.user_id ");
    query.push_str("join golfer as g on g.golfer_id = eup.golfer_id ");
    query.push_str("join event as e on e.event_id = eup.event_id ");
    query.push_str("where e.espn_id = ?1;");
    let res = conn
        .execute_select(&query, &[RowValues::Int(401580351)])
        .await?;

    assert_eq!(res.results.len(), 15);
    let x = res.results.iter().find(|z| {
        z.get("bettor").unwrap().as_text().unwrap() == "Player4"
            && *z.get("golfer_espn_id").unwrap().as_int().unwrap() == 9780
    });
    assert!(x.is_some());

    let mut query = "select b.name as bettor, g.espn_id as golfer_espn_id ".to_string();
    query.push_str("from event_user_player as eup ");
    query.push_str("join bettor as b on b.user_id = eup.user_id ");
    query.push_str("join golfer as g on g.golfer_id = eup.golfer_id ");
    query.push_str("join event as e on e.event_id = eup.event_id ");
    query.push_str("where e.espn_id = ?1;");
    let res = conn
        .execute_select(&query, &[RowValues::Int(401580360)])
        .await?;

    assert_eq!(res.results.len(), 15);
    let x = res.results.iter().find(|z| {
        z.get("bettor").unwrap().as_text().unwrap() == "Player3"
            && *z.get("golfer_espn_id").unwrap().as_int().unwrap() == 4364873
    });
    assert!(x.is_some());

    Ok(())
}
