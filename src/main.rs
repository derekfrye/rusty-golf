// extern crate no longer needed in Rust 2018+
use deadpool_postgres::{ManagerConfig, RecyclingMethod};
use rusty_golf::args;
use rusty_golf::controller::{db_prefill, score::scores};
use rusty_golf::model::get_event_details;
use sql_middleware::middleware::{ConfigAndPool, DatabaseType};

use actix_files::Files;
use actix_web::web::Data;
use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use std::collections::HashMap;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = args::args_checks();
    let args_for_web = args.clone();

    let (config_and_pool, db_type) = init_config_and_pool(&args).await?;
    run_startup_tasks(&args, &config_and_pool, db_type).await?;

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(config_and_pool.clone()))
            .app_data(Data::new(args_for_web.clone()))
            .route("/", web::get().to(index))
            .route("/scores", web::get().to(scores))
            .route(
                "/scores/summary",
                web::get().to(rusty_golf::controller::score::http_handlers::scores_summary),
            )
            .route(
                "/scores/chart",
                web::get().to(rusty_golf::controller::score::http_handlers::scores_chart),
            )
            .route(
                "/scores/linescore",
                web::get().to(rusty_golf::controller::score::http_handlers::scores_linescore),
            )
            .route("/health", web::get().to(HttpResponse::Ok))
            .service(Files::new("/static", "./static").show_files_listing()) // Serve the static files
    })
    .bind("0.0.0.0:5201")?
    .run()
    .await?;
    Ok(())
}

async fn index(
    query: web::Query<HashMap<String, String>>,
    abc: Data<ConfigAndPool>,
) -> impl Responder {
    // let db = Db::new(abc.get_ref().clone()).unwrap();
    let config_and_pool = abc.get_ref().clone();
    let event_str = query.get("event").unwrap_or(&String::new()).to_string();

    let title = match event_str.parse() {
        Ok(id) => match get_event_details(&config_and_pool, id).await {
            Ok(event_config) => event_config.event_name,
            Err(e) => {
                eprintln!("Error: {e}");
                "Scoreboard".to_string()
            }
        },
        Err(e) => {
            eprintln!("Error: {e}");
            "Scoreboard".to_string()
        }
    };

    let markup = rusty_golf::view::index::render_index_template(&title);
    HttpResponse::Ok()
        .content_type("text/html")
        .body(markup.into_string())
}

async fn init_config_and_pool(
    args: &args::CleanArgs,
) -> Result<(ConfigAndPool, DatabaseType), Box<dyn std::error::Error>> {
    if args.db_type == DatabaseType::Postgres {
        let mut postgres_config = deadpool_postgres::Config::new();
        postgres_config.dbname = Some(args.db_name.clone());
        postgres_config.host = args.db_host.clone();
        postgres_config.port = args.db_port;
        postgres_config.user = args.db_user.clone();
        postgres_config.password = args.db_password.clone();
        postgres_config.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        let pool = ConfigAndPool::new_postgres(postgres_config).await?;
        Ok((pool, DatabaseType::Postgres))
    } else {
        match ConfigAndPool::new_sqlite(args.db_name.clone()).await {
            Ok(pool) => Ok((pool, DatabaseType::Sqlite)),
            Err(e) => {
                eprintln!(
                    "Error: {}\nBacktrace: {:?}",
                    e,
                    std::backtrace::Backtrace::capture()
                );
                std::process::exit(1);
            }
        }
    }
}

async fn run_startup_tasks(
    args: &args::CleanArgs,
    config_and_pool: &ConfigAndPool,
    db_type: DatabaseType,
) -> Result<(), Box<dyn std::error::Error>> {
    if args.db_startup_script.is_some() {
        rusty_golf::model::execute_batch_sql(config_and_pool, &args.combined_sql_script).await?;
    }

    if let Some(json_data) = &args.db_populate_json {
        db_prefill::db_prefill(json_data, config_and_pool, db_type).await?;
    }

    Ok(())
}
