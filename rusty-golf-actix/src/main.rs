use rusty_golf::args;
use rusty_golf::controller::{db_prefill, score::scores};
use rusty_golf::view::index::{DEFAULT_INDEX_TITLE, try_resolve_index_title};
use rusty_golf::storage::SqlStorage;
use sql_middleware::middleware::{
    ConfigAndPool, DatabaseType, PgConfig, PostgresOptions, SqliteOptions,
};

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

    let storage = SqlStorage::new(config_and_pool.clone());

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(storage.clone()))
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
    storage: Data<SqlStorage>,
) -> impl Responder {
    let event_str = query.get("event").cloned().unwrap_or_default();

    let title = match try_resolve_index_title(storage.get_ref(), &event_str).await {
        Ok(title) => title,
        Err(e) => {
            eprintln!("Error: {e}");
            DEFAULT_INDEX_TITLE.to_string()
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
        let mut postgres_config = PgConfig::new();
        postgres_config.dbname = Some(args.db_name.clone());
        postgres_config.host.clone_from(&args.db_host);
        postgres_config.port = args.db_port;
        postgres_config.user.clone_from(&args.db_user);
        postgres_config.password.clone_from(&args.db_password);

        let postgres_options = PostgresOptions::new(postgres_config);
        let pool = ConfigAndPool::new_postgres(postgres_options).await?;
        Ok((pool, DatabaseType::Postgres))
    } else {
        let sqlite_options = SqliteOptions::new(args.db_name.clone());
        match ConfigAndPool::new_sqlite(sqlite_options).await {
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
