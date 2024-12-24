use rusty_golf::admin::router;
use rusty_golf::controller::score::scores;
use sqlx_middleware::db::db::{DatabaseSetupState, DatabaseType, Db, DbConfigAndPool};
use deadpool_postgres::{ManagerConfig, RecyclingMethod};
use rusty_golf::model::{get_title_from_db, CacheMap};

use actix_web::web::Data;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
// use chrono::{DateTime, Utc};
use actix_files::Files;
// use serde_json::json;
// use tokio_postgres::config;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use tokio::sync::RwLock;



#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let dotenv_path = dotenv::dotenv();

    // Example fix for let_and_return: directly return the result
    if let Ok(path) = dotenv_path {
        dbg!(
            path.to_str(),
            dotenv::var("TOKEN").unwrap(),
            dotenv::var("DB_HOST").unwrap(),
            dotenv::var("DB_PORT").unwrap()
        );
    }

    let mut db_pwd = env::var("DB_PASSWORD").unwrap_or_default();
    if db_pwd == "/secrets/db_password" {
        // open the file and read the contents
        let contents = std::fs::read_to_string("/secrets/db_password")
            .unwrap_or("tempPasswordWillbeReplacedIn!AdminPanel".to_string());
        // set the password to the contents of the file
        db_pwd = contents.trim().to_string();
    }
    let mut cfg = deadpool_postgres::Config::new();
    cfg.dbname = Some(env::var("DB_NAME").unwrap_or_default());
    cfg.host = Some(env::var("DB_HOST").unwrap_or_default());
    cfg.port = Some(env::var("DB_PORT").unwrap_or_default().parse::<u16>().unwrap_or_default());
    cfg.user = Some(env::var("DB_USER").unwrap_or_default());
    cfg.password = Some(db_pwd);

    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    let dbcn = DbConfigAndPool::new(cfg, DatabaseType::Postgres).await;


    let mut cfg2 = deadpool_postgres::Config::new();
    cfg2.dbname = Some("test.db".to_string());
    let sqlitex = DbConfigAndPool::new(cfg2, DatabaseType::Sqlite).await;
    let _db2 = Db::new(sqlitex).unwrap();

    let cache_map: CacheMap = Arc::new(RwLock::new(HashMap::new()));

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(cache_map.clone()))
            .app_data(Data::new(dbcn.clone()))
            .route("/", web::get().to(index))
            .route("/scores", web::get().to(scores))
            .route("/admin", web::get().to(admin))
            .service(Files::new("/static", "./static").show_files_listing()) // Serve the static files
    })
    .bind("0.0.0.0:8081")?
    .run()
    .await
}

async fn index(
    query: web::Query<HashMap<String, String>>,
    abc: Data<DbConfigAndPool>,
) -> impl Responder {
    let db = Db::new(abc.get_ref().clone()).unwrap();
    let event_str = query
        .get("event")
        .unwrap_or(&String::new())
        .to_string();

    let mut title = "Scoreboard".to_string();
    let _: i32 = match event_str.parse() {
        Ok(id) => {
            let title_test = get_title_from_db(&db, id).await;
            match title_test {
                Ok(t) => {
                    if t.db_last_exec_state == DatabaseSetupState::QueryReturnedSuccessfully
                    {
                        title = t.return_result.clone();
                    }
                }
                Err(ref x) => {
                    eprintln!("Error: {}", x);
                }
            }
            id
        }
        Err(_) => {
            0 // or any default value you prefer
        }
    };

    let markup = rusty_golf::view::index::render_index_template(title);
    HttpResponse::Ok()
        .content_type("text/html")
        .body(markup.into_string())
}


async fn admin(
    query: web::Query<HashMap<String, String>>,
    abc: Data<DbConfigAndPool>,
) -> HttpResponse {
    let db = Db::new(abc.get_ref().clone()).unwrap();
    let mut router = router::AdminRouter::new();
    // let mut db = Db::new(abc.get_ref().clone()).unwrap();
    router.router(query, db).await
}
