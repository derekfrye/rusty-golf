mod common;
use crate::common::ConnExt;
use crate::db_prefill::db_prefill;
use rusty_golf_actix::controller::db_prefill;
// `, `use rusty_golf_actix::controller::db_prefill::db_prefill;

// use rusty_golf_actix::controller::score;

use sql_middleware::middleware::{
    ConfigAndPool as ConfigAndPool2, DatabaseType, QueryAndParams, RowValues, SqliteOptions,
};
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn test_dbprefill() -> Result<(), Box<dyn std::error::Error>> {
    let config_and_pool = setup_sqlite().await?;
    let mut conn = config_and_pool.get_connection().await?;

    assert_empty_tables(&mut conn).await?;

    let json = serde_json::from_str(include_str!("test05_dbprefill.json"))?;
    db_prefill(&json, &config_and_pool, DatabaseType::Sqlite).await?;

    assert_events(&mut conn).await?;
    assert_golfers(&mut conn).await?;
    assert_bettors(&mut conn).await?;
    assert_event_user_players(&mut conn, 401_580_351, "Player4", 9780).await?;
    assert_event_user_players(&mut conn, 401_580_360, "Player3", 4_364_873).await?;

    Ok(())
}

async fn setup_sqlite() -> Result<ConfigAndPool2, Box<dyn std::error::Error>> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time went backwards")
        .as_nanos();
    let db_path = format!("file:test_db_{unique}?mode=memory&cache=shared");
    let sqlite_options = SqliteOptions::new(db_path);
    let config_and_pool = ConfigAndPool2::new_sqlite(sqlite_options).await.unwrap();

    let ddl = [
        include_str!("../../actix/src/sql/schema/sqlite/00_event.sql"),
        // include_str!("../../actix/src/sql/schema/sqlite/01_golfstatistic.sql"),
        include_str!("../../actix/src/sql/schema/sqlite/02_golfer.sql"),
        include_str!("../../actix/src/sql/schema/sqlite/03_bettor.sql"),
        include_str!("../../actix/src/sql/schema/sqlite/04_event_user_player.sql"),
        include_str!("../../actix/src/sql/schema/sqlite/05_eup_statistic.sql"),
    ];

    let query_and_params = QueryAndParams {
        query: ddl.join("\n"),
        params: vec![],
    };

    let mut conn = config_and_pool.get_connection().await?;
    conn.execute_batch(&query_and_params.query).await?;

    Ok(config_and_pool)
}

async fn assert_empty_tables(conn: &mut impl ConnExt) -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}

async fn assert_events(conn: &mut impl ConnExt) -> Result<(), Box<dyn std::error::Error>> {
    let query = "select * from event ;";
    let res = conn.execute_select(query, &[]).await?;
    // test05_dbprefill.json currently contains 5 events
    assert_eq!(res.results.len(), 5);
    Ok(())
}

async fn assert_golfers(conn: &mut impl ConnExt) -> Result<(), Box<dyn std::error::Error>> {
    let query = "select * from golfer;";
    let res = conn.execute_select(query, &[]).await?;
    // The test05_dbprefill.json file across all events currently lists 33 unique golfers.
    // Duplicates are handled by the INSERT statement that uses "WHERE NOT EXISTS".
    assert_eq!(res.results.len(), 33);
    let golfer = res
        .results
        .iter()
        .find(|z| *z.get("espn_id").unwrap().as_int().unwrap() == 4_375_972);
    assert!(golfer.is_some());
    assert_eq!(
        golfer.unwrap().get("name").unwrap().as_text().unwrap(),
        "Ludvig Åberg"
    );
    Ok(())
}

async fn assert_bettors(conn: &mut impl ConnExt) -> Result<(), Box<dyn std::error::Error>> {
    let query = "select * from bettor;";
    let res = conn.execute_select(query, &[]).await?;
    // Two naming styles exist in the fixture: "PlayerN" and "Player N".
    // This yields 10 distinct bettors total.
    assert_eq!(res.results.len(), 10);
    let bettor = res
        .results
        .iter()
        .find(|z| z.get("name").unwrap().as_text().unwrap() == "Player5");
    assert!(bettor.is_some());
    let bettor_spaced = res
        .results
        .iter()
        .find(|z| z.get("name").unwrap().as_text().unwrap() == "Player 5");
    assert!(bettor_spaced.is_some());
    Ok(())
}

async fn assert_event_user_players(
    conn: &mut impl ConnExt,
    event_id: i64,
    expected_bettor: &str,
    expected_golfer: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut query = "select b.name as bettor, g.espn_id as golfer_espn_id ".to_string();
    query.push_str("from event_user_player as eup ");
    query.push_str("join bettor as b on b.user_id = eup.user_id ");
    query.push_str("join golfer as g on g.golfer_id = eup.golfer_id ");
    query.push_str("join event as e on e.event_id = eup.event_id ");
    query.push_str("where e.espn_id = ?1;");
    let res = conn
        .execute_select(&query, &[RowValues::Int(event_id)])
        .await?;

    assert_eq!(res.results.len(), 15);
    let hit = res.results.iter().find(|z| {
        z.get("bettor").unwrap().as_text().unwrap() == expected_bettor
            && *z.get("golfer_espn_id").unwrap().as_int().unwrap() == expected_golfer
    });
    assert!(hit.is_some());
    Ok(())
}
