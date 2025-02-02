extern crate rusty_golf;
use deadpool_postgres::{ ManagerConfig, RecyclingMethod };
use rusty_golf::admin::router;
use rusty_golf::controller::{ db_prefill, score::scores };
use rusty_golf::model::{ get_title_from_db, CacheMap };
use sql_middleware::middleware::{
    ConfigAndPool,
    DatabaseType,
    MiddlewarePool,
    MiddlewarePoolConnection,
    QueryAndParams,
};

use actix_files::Files;
use actix_web::web::Data;
use actix_web::{ web, App, HttpResponse, HttpServer, Responder };
use sql_middleware::SqlMiddlewareDbError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

mod args;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = args::args_checks();

    let mut cfg = deadpool_postgres::Config::new();
    let config_and_pool: ConfigAndPool;
    let db_type: DatabaseType;
    // let pth = "file::memory:?cache=shared".to_string();
    // let cfg2 = ConfigAndPool::new_sqlite(pth).await.unwrap();
    if args.db_type == DatabaseType::Postgres {
        cfg.dbname = Some(args.db_name);
        cfg.host = args.db_host;
        cfg.port = args.db_port;
        cfg.user = args.db_user;
        cfg.password = args.db_password;
        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });
        config_and_pool = ConfigAndPool::new_postgres(cfg).await?;
        db_type = DatabaseType::Postgres;
    } else {
        config_and_pool = ConfigAndPool::new_sqlite(args.db_name).await?;
        // let sqlite_configandpool = ConfigAndPool::new_sqlite(x).await.unwrap();
        // let pool = sqlite_configandpool.pool.get().await.unwrap();
        // let conn = MiddlewarePool::get_connection(pool).await.unwrap();
        db_type = DatabaseType::Sqlite;
    }

    if args.db_startup_script.is_some() {
        // let db = Db::new(dbcn.clone()).unwrap();
        let script = args.combined_sql_script;
        // db.execute(&script).await.unwrap();
        let query_and_params = QueryAndParams {
            query: script,
            params: vec![],
        };

        let pool = config_and_pool.pool.get().await?;
        let sconn = MiddlewarePool::get_connection(pool).await?;
        (match sconn {
            MiddlewarePoolConnection::Postgres(mut xx) => {
                let tx = xx.transaction().await?;

                tx.batch_execute(&query_and_params.query).await?;
                tx.commit().await?;
                Ok::<_, SqlMiddlewareDbError>(())
            }
            MiddlewarePoolConnection::Sqlite(xx) => {
                xx.interact(move |xxx| {
                    let tx = xxx.transaction()?;
                    tx.execute_batch(&query_and_params.query)?;

                    tx.commit()?;
                    Ok::<_, SqlMiddlewareDbError>(())
                }).await?
            }
        })?;
    }

    if args.db_populate_json.is_some() {
        let _res = db_prefill::db_prefill(
            &args.db_populate_json.unwrap(),
            &config_and_pool,
            db_type
        ).await?;
        // db_prefill(args.db_populate_json.unwrap());
    }

    let cache_map: CacheMap = Arc::new(RwLock::new(HashMap::new()));

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(cache_map.clone()))
            .app_data(Data::new(config_and_pool.clone()))
            .route("/", web::get().to(index))
            .route("/scores", web::get().to(scores))
            .route("/admin", web::get().to(admin))
            .route(
                "/health",
                web::get().to(|| HttpResponse::Ok())
            )
            .service(Files::new("/static", "./static").show_files_listing()) // Serve the static files
    })
        .bind("0.0.0.0:8081")?
        .run().await?;
    Ok(())
}

async fn index(
    query: web::Query<HashMap<String, String>>,
    abc: Data<ConfigAndPool>
) -> impl Responder {
    // let db = Db::new(abc.get_ref().clone()).unwrap();
    let config_and_pool = abc.get_ref().clone();
    let event_str = query.get("event").unwrap_or(&String::new()).to_string();

    let mut title = "Scoreboard".to_string();
    let _: i32 = match event_str.parse() {
        Ok(id) => {
            let title_test = get_title_from_db(&config_and_pool, id).await;
            match title_test {
                Ok(t) => {
                    title = t;
                }
                Err(_) => {
                    // eprintln!("Error: {}", x);
                }
            }
            id
        }
        Err(_) => {
            0 // or any default value you prefer
        }
    };

    let markup = rusty_golf::view::index::render_index_template(title);
    HttpResponse::Ok().content_type("text/html").body(markup.into_string())
}

async fn admin(
    query: web::Query<HashMap<String, String>>,
    abc: Data<ConfigAndPool>
) -> Result<HttpResponse, actix_web::Error> {
    // let db = Db::new(abc.get_ref().clone()).unwrap();
    let config_and_pool = abc.get_ref().clone();
    let mut router = router::AdminRouter::new();
    // let mut db = Db::new(abc.get_ref().clone()).unwrap();
    router.router(query, config_and_pool).await
}
