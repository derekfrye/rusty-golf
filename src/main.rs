extern crate rusty_golf; 
use deadpool_postgres::{ManagerConfig, RecyclingMethod};
use rusty_golf::admin::router;
use rusty_golf::controller::{score::scores, db_prefill};

use rusty_golf::model::{get_title_from_db, CacheMap};
use sqlx_middleware::db::{ConfigAndPool, DatabaseType, Db, QueryState};
use sqlx_middleware::middleware::ConfigAndPool as ConfigAndPool2;

use actix_web::web::Data;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_files::Files;
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

mod args;

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let args = args::args_checks();

    let mut cfg = deadpool_postgres::Config::new();
    let dbcn: ConfigAndPool;
    let pth =  "file::memory:?cache=shared".to_string();
    let cfg2 = ConfigAndPool2::new_sqlite(pth).await.unwrap();
    if args.db_type == DatabaseType::Postgres {
        cfg.dbname = Some(args.db_name);
        cfg.host = args.db_host;
        cfg.port = args.db_port;
        cfg.user = args.db_user;
        cfg.password = args.db_password;
        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });
        dbcn = ConfigAndPool::new(cfg, DatabaseType::Postgres).await;
    } else {
        cfg.dbname = Some(args.db_name);
        dbcn = ConfigAndPool::new(cfg, DatabaseType::Sqlite).await;
        // let sqlite_configandpool = ConfigAndPool2::new_sqlite(x).await.unwrap();
    // let pool = sqlite_configandpool.pool.get().await.unwrap();
    // let conn = MiddlewarePool::get_connection(pool).await.unwrap();
    }

    if args.db_startup_script.is_some() {
        let db = Db::new(dbcn.clone()).unwrap();
        let script = args.combined_sql_script;
        // db.execute(&script).await.unwrap();
        let query_and_params = sqlx_middleware::model::QueryAndParams {
            query: script,
            params: vec![],
        };
        let result = db.exec_general_query(vec![query_and_params], false).await;
        if result.is_err() || result.as_ref().unwrap().db_last_exec_state != QueryState::QueryReturnedSuccessfully {
            let res_err = if result.is_err() {
                result.err().unwrap().to_string()
            } else {
                "".to_string()
            };
            eprintln!("Fatal Error on db startup. {}", res_err);
            eprintln!("Check your --db-startup-script, since you passed that var.");
            std::process::exit(1);
        }
    }

    if args.db_populate_json.is_some() {
        let _res=db_prefill::db_prefill(&args.db_populate_json.unwrap(), &cfg2 );
        // db_prefill(args.db_populate_json.unwrap());
    }

    let cache_map: CacheMap = Arc::new(RwLock::new(HashMap::new()));

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(cache_map.clone()))
            .app_data(Data::new(dbcn.clone()))
            .route("/", web::get().to(index))
            .route("/scores", web::get().to(scores))
            .route("/admin", web::get().to(admin))
            .route("/health", web::get().to(|| HttpResponse::Ok()))
            .service(Files::new("/static", "./static").show_files_listing()) // Serve the static files
    })
    .bind("0.0.0.0:8081")?
    .run()
    .await
}

async fn index(
    query: web::Query<HashMap<String, String>>,
    abc: Data<ConfigAndPool>,
) -> impl Responder {
    let db = Db::new(abc.get_ref().clone()).unwrap();
    let event_str = query.get("event").unwrap_or(&String::new()).to_string();

    let mut title = "Scoreboard".to_string();
    let _: i32 = match event_str.parse() {
        Ok(id) => {
            let title_test = get_title_from_db(&db, id).await;
            match title_test {
                Ok(t) => {
                    if t.db_last_exec_state == QueryState::QueryReturnedSuccessfully {
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
    abc: Data<ConfigAndPool>,
) -> HttpResponse {
    let db = Db::new(abc.get_ref().clone()).unwrap();
    let mut router = router::AdminRouter::new();
    // let mut db = Db::new(abc.get_ref().clone()).unwrap();
    router.router(query, db).await
}
