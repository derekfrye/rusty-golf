use crate::{ResultStatus, Scores, Statistic};
use std::env;
use tokio_postgres::{Config, NoTls};

pub async fn get_golfers_from_db(event_id: i32) -> Result<Vec<Scores>, Box<dyn std::error::Error>> {
    let db_user = env::var("DB_USER")?;
    let mut db_password = env::var("DB_PASSWORD")?;
    let db_host = env::var("DB_HOST")?;
    let db_name = env::var("DB_NAME")?;
    let db_port = env::var("DB_PORT")?;

    if db_password == "/secrets/db_password" {
        // open the file and read the contents
        let contents = std::fs::read_to_string("/secrets/db_password")?;
        // set the password to the contents of the file
        db_password = contents.trim().to_string();
    }

    // let db_url = format!(
    //     "postgres://{}:{}@{}:{}/{}",
    //     db_user, db_password, db_host, db_port, db_name
    // );

    let (client, conn) = Config::new()
        .host(&db_host)
        .port(db_port.parse::<u16>().unwrap())
        .user(&db_user)
        .password(db_password)
        .dbname(&db_name)
        .connect(NoTls)
        .await?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("connection error: {}", e);
        }
    });

    // let conn = tokio_postgres::connect(&db_url, tokio_postgres::NoTls)
    //     .await?
    //     .0;

    let rows = client
        .query("SELECT grp, golfername, playername, eup_id, espn_id FROM sp_get_player_names($1) ORDER BY grp, eup_id", &[&event_id])
        .await?;

    let players = rows
        .iter()
        .map(|row| Scores {
            // parse column 0 as an int32
            group: row.get::<_, i64>(0),
            golfer_name: row.get(1),
            bettor_name: row.get(2),
            eup_id: row.get::<_, i64>(3),
            espn_id: row.get::<_, i64>(4),
            detailed_statistics: Statistic {
                eup_id: row.get::<_, i64>(3),
                rounds: vec![],
                scores: vec![],
                tee_times: vec![],
                holes_completed: vec![],
                success_fail: ResultStatus::NoData,
                total_score: 0,
            },
        })
        .collect();

    Ok(players)
}
