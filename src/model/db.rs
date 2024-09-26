use crate::model::model::{ResultStatus, Scores, Statistic};
use std::env;
use tokio_postgres::{tls::NoTlsStream, Config, NoTls, Socket};

struct ConnectionParams {
    db_user: String,
    db_password: String,
    db_host: String,
    db_name: String,
    db_port: String,
}

impl ConnectionParams {
    pub fn new() -> Self {
        ConnectionParams {
            db_user: env::var("DB_USER").unwrap(),
            db_password: env::var("DB_PASSWORD").unwrap(),
            db_host: env::var("DB_HOST").unwrap(),
            db_name: env::var("DB_NAME").unwrap(),
            db_port: env::var("DB_PORT").unwrap(),
        }
    }

    fn read_password_from_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.db_password == "/secrets/db_password" {
            // open the file and read the contents
            let contents = std::fs::read_to_string("/secrets/db_password")?;
            // set the password to the contents of the file
            self.db_password = contents.trim().to_string();
        }

        Ok(())
    }

    pub async fn return_client_and_connection(
        &self,
    ) -> Result<
        (
            tokio_postgres::Client,
            tokio_postgres::Connection<Socket, NoTlsStream>,
        ),
        Box<dyn std::error::Error>,
    > {
        let (client, conn) = Config::new()
            .host(&self.db_host)
            .port(self.db_port.parse::<u16>().unwrap())
            .user(&self.db_user)
            .password(self.db_password.clone())
            .dbname(&self.db_name)
            .connect(NoTls)
            .await?;

        Ok((client, conn))
    }
}

pub async fn get_title_from_db(event_id: i32) -> Result<String, Box<dyn std::error::Error>> {
    let mut conn_params = ConnectionParams::new();
    conn_params.read_password_from_file()?;
    let (client, conn) = conn_params.return_client_and_connection().await?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("connection error: {}", e);
        }
    });

    // let conn = tokio_postgres::connect(&db_url, tokio_postgres::NoTls)
    //     .await?
    //     .0;

    let row = client
        .query_one("SELECT eventname FROM sp_get_event_name($1)", &[&event_id])
        .await?;

    let title: String = row.get(0);

    Ok(title)
}

pub async fn get_golfers_from_db(event_id: i32) -> Result<Vec<Scores>, Box<dyn std::error::Error>> {
    let mut conn_params = ConnectionParams::new();
    conn_params.read_password_from_file()?;
    let (client, conn) = conn_params.return_client_and_connection().await?;

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
