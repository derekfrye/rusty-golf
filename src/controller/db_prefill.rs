use serde_json::Value;
use sql_middleware::{
    SqlMiddlewareDbError,
    middleware::{ConfigAndPool, DatabaseType, MiddlewarePoolConnection, RowValues},
};

/// # Errors
///
/// Will return `Err` if the database query fails
pub async fn db_prefill(
    json: &Value,
    config_and_pool: &ConfigAndPool,
    db_type: DatabaseType,
) -> Result<(), SqlMiddlewareDbError> {
    let conn = config_and_pool.get_connection().await?;

    match db_type {
        DatabaseType::Sqlite => prefill_sqlite(conn, json).await?,
        DatabaseType::Postgres => unimplemented!(),
    }
    Ok(())
}

async fn prefill_sqlite(
    mut conn: MiddlewarePoolConnection,
    json: &Value,
) -> Result<(), SqlMiddlewareDbError> {
    if cfg!(debug_assertions) {
        let _pretty_json = serde_json::to_string_pretty(json).unwrap();
    }

    let data = json.as_array().unwrap();

    prefill_sqlite_inner(&mut conn, data).await
}

async fn prefill_sqlite_inner(
    conn: &mut MiddlewarePoolConnection,
    data: &[Value],
) -> Result<(), SqlMiddlewareDbError> {
    for datum in data {
        process_event_datum(conn, datum).await?;
    }
    Ok(())
}

async fn process_event_datum(
    conn: &mut MiddlewarePoolConnection,
    datum: &Value,
) -> Result<(), SqlMiddlewareDbError> {
    let espn_id = datum["event"].as_i64().unwrap();
    let year = datum["year"].as_i64().unwrap();

    if event_exists(conn, espn_id, year).await? {
        println!("Event {espn_id} and year {year} already exist in the db. Skipping db prefill.");
    } else {
        insert_event(conn, datum).await?;
        let data_to_fill = datum["data_to_fill_if_event_and_year_missing"]
            .as_array()
            .unwrap();
        for data in data_to_fill {
            insert_bettors(conn, data["bettors"].as_array().unwrap()).await?;
            insert_golfers(conn, data["golfers"].as_array().unwrap()).await?;
            insert_event_user_players(conn, data["event_user_player"].as_array().unwrap(), datum)
                .await?;
        }
    }
    Ok(())
}

async fn event_exists(
    conn: &mut MiddlewarePoolConnection,
    espn_id: i64,
    year: i64,
) -> Result<bool, SqlMiddlewareDbError> {
    let params = [RowValues::Int(espn_id), RowValues::Int(year)];
    let result_set = conn
        .query("SELECT 1 FROM event WHERE espn_id = ?1 AND year = ?2;")
        .params(&params)
        .select()
        .await?;
    Ok(!result_set.results.is_empty())
}

async fn insert_event(
    conn: &mut MiddlewarePoolConnection,
    datum: &Value,
) -> Result<(), SqlMiddlewareDbError> {
    let params = [
        RowValues::Text(datum["name"].as_str().unwrap().to_string()),
        RowValues::Int(datum["event"].as_i64().unwrap()),
        RowValues::Int(datum["year"].as_i64().unwrap()),
        RowValues::Float(datum["score_view_step_factor"].as_f64().unwrap()),
    ];
    conn.query(
        "INSERT INTO event (name, espn_id, year, score_view_step_factor) VALUES(?1, ?2, ?3, ?4);",
    )
    .params(&params)
    .dml()
    .await?;
    Ok(())
}

async fn insert_bettors(
    conn: &mut MiddlewarePoolConnection,
    bettors: &[Value],
) -> Result<(), SqlMiddlewareDbError> {
    for bettor in bettors {
        let params = [RowValues::Text(bettor.as_str().unwrap().to_string())];
        conn.query("INSERT INTO bettor (name) SELECT ?1 WHERE NOT EXISTS (SELECT 1 from bettor where name = ?1);")
            .params(&params)
            .dml()
            .await?;
    }
    Ok(())
}

async fn insert_golfers(
    conn: &mut MiddlewarePoolConnection,
    golfers: &[Value],
) -> Result<(), SqlMiddlewareDbError> {
    for golfer in golfers {
        let params = [
            RowValues::Text(golfer["name"].as_str().unwrap().to_string()),
            RowValues::Int(golfer["espn_id"].as_i64().unwrap()),
        ];
        conn.query("INSERT INTO golfer (name, espn_id) SELECT ?1, ?2 WHERE NOT EXISTS (SELECT 1 from golfer where espn_id = ?2);")
            .params(&params)
            .dml()
            .await?;
    }
    Ok(())
}

async fn insert_event_user_players(
    conn: &mut MiddlewarePoolConnection,
    event_user_players: &[Value],
    datum: &Value,
) -> Result<(), SqlMiddlewareDbError> {
    for event_user_player in event_user_players {
        let mut params = vec![
            RowValues::Int(datum["event"].as_i64().unwrap()),
            RowValues::Text(event_user_player["bettor"].as_str().unwrap().to_string()),
            RowValues::Int(event_user_player["golfer_espn_id"].as_i64().unwrap()),
        ];

        let mut query_columns = "(event_id, user_id, golfer_id".to_string();
        let mut query_values =
            " select (select event_id from event where espn_id = ?1),".to_string();
        query_values.push_str("(select user_id from bettor where name = ?2),");
        query_values.push_str("(select golfer_id from golfer where espn_id = ?3)");

        if event_user_player.get("score_view_step_factor").is_some() {
            query_columns.push_str(", score_view_step_factor");
            query_values.push_str(", ?4");
            params.push(RowValues::Float(
                event_user_player["score_view_step_factor"]
                    .as_f64()
                    .unwrap(),
            ));
        } else {
            query_columns.push_str(", score_view_step_factor");
            query_values.push_str(", NULL");
        }

        query_columns.push(')');
        query_values.push(';');

        let query = format!("INSERT INTO event_user_player {query_columns}{query_values}");
        conn.query(&query).params(&params).dml().await?;
    }
    Ok(())
}
