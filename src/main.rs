// extern crate no longer needed in Rust 2018+
use deadpool_postgres::{ ManagerConfig, RecyclingMethod };
use rusty_golf::admin::router;
use rusty_golf::args;
use rusty_golf::controller::{ db_prefill, score::scores };
use rusty_golf::model::get_event_details;
use sql_middleware::middleware::{
    ConfigAndPool,
    DatabaseType,
    MiddlewarePool,
    MiddlewarePoolConnection,
    QueryAndParams,
};

use actix_files::Files;
use actix_web::web::Data;
use actix_web::{ App, HttpResponse, HttpServer, Responder, web };
use sql_middleware::SqlMiddlewareDbError;
use std::collections::HashMap;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = args::args_checks();
    let args_for_web = args.clone();

    let cfg = deadpool_postgres::Config::new();
    let config_and_pool: ConfigAndPool;
    let db_type: DatabaseType;
    // let pth = "file::memory:?cache=shared".to_string();
    // let cfg2 = ConfigAndPool::new_sqlite(pth).await.unwrap();
    if args.db_type == DatabaseType::Postgres {
        let mut postgres_config = cfg;
        postgres_config.dbname = Some(args.db_name);
        postgres_config.host = args.db_host;
        postgres_config.port = args.db_port;
        postgres_config.user = args.db_user;
        postgres_config.password = args.db_password;
        postgres_config.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });
        config_and_pool = ConfigAndPool::new_postgres(postgres_config).await?;
        db_type = DatabaseType::Postgres;
    } else {
        let a = ConfigAndPool::new_sqlite(args.db_name).await;
        match a {
            Ok(a) => {
                config_and_pool = a;
            }
            Err(e) => {
                eprintln!("Error: {}\nBacktrace: {:?}", e, std::backtrace::Backtrace::capture());
                std::process::exit(1);
            }
        }
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
            } // MiddlewarePoolConnection::Mssql(_) => todo!()
        })?;
    }

    if let Some(json_path) = &args.db_populate_json {
        db_prefill::db_prefill(json_path, &config_and_pool, db_type).await?;
    }

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(config_and_pool.clone()))
            .app_data(Data::new(args_for_web.clone()))
            .route("/", web::get().to(index))
            .route("/scores", web::get().to(scores))
            .route("/admin", web::get().to(admin))
            .route("/health", web::get().to(HttpResponse::Ok))
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

    let title = match event_str.parse() {
        Ok(id) =>
            match get_event_details(&config_and_pool, id).await {
                Ok(event_config) => event_config.event_name,
                Err(e) => {
                    eprintln!("Error: {e}");
                    "Scoreboard".to_string()
                }
            }
        Err(e) => {
            eprintln!("Error: {e}");
            "Scoreboard".to_string()
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
