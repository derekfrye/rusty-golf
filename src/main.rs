mod model;
mod db;
mod admin {
    pub mod view {
        pub mod admin00_landing;
        pub mod admin01_tables;
        pub mod admin0x;
    }
    pub mod model {
        pub mod admin_model;
    }
    pub mod router;
}
mod controller {
    pub mod cache;
    pub mod espn;
    pub mod score;
}
mod view {
    pub mod index;
    pub mod score;
}

use admin::router;
use model::CacheMap;

use actix_web::web::Data;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
// use chrono::{DateTime, Utc};
use actix_files::Files;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

const HTMX_PATH: &str = "https://unpkg.com/htmx.org@1.9.12";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let dotenv_path = dotenv::dotenv();
    // print the filename it loaded from
    if dotenv::dotenv().is_ok() {
        dbg!(
            dotenv_path.unwrap().to_str(),
            dotenv::var("TOKEN").unwrap(),
            dotenv::var("DB_HOST").unwrap(),
            dotenv::var("DB_PORT").unwrap()
        );
    }

    let cache_map: CacheMap = Arc::new(RwLock::new(HashMap::new()));

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(cache_map.clone()))
            .route("/", web::get().to(index))
            .route("/scores", web::get().to(scores))
            .route("/admin", web::get().to(admin))
            .service(Files::new("/static", "./static").show_files_listing()) // Serve the static files
    })
    .bind("0.0.0.0:8081")?
    .run()
    .await
}

async fn index(query: web::Query<HashMap<String, String>>) -> impl Responder {
    let event_str = query
        .get("event")
        .unwrap_or(&String::new())
        .trim()
        .to_string();
    let mut title = "Scoreboard".to_string();
    let _: i32 = match event_str.parse() {
        Ok(id) => {
            let title_test = db::get_title_from_db(id).await;
            match title_test {
                Ok(t) => {
                    if t.db_last_exec_state == db::DatabaseSetupState::QueryReturnedSuccessfully {
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

    let markup = crate::view::index::render_index_template(title);
    HttpResponse::Ok()
        .content_type("text/html")
        .body(markup.into_string())
}

async fn scores(
    cache_map: Data<CacheMap>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let event_str = query
        .get("event")
        .unwrap_or(&String::new())
        .trim()
        .to_string();
    let event_id: i32 = match event_str.parse() {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": "espn event parameter is required"}));
        }
    };

    let year_str = query.get("yr").unwrap_or(&String::new()).trim().to_string();
    let year: i32 = match year_str.parse() {
        Ok(y) => y,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": "yr (year) parameter is required"}));
        }
    };

    let cache_str = query
        .get("cache")
        .unwrap_or(&String::new())
        .trim()
        .to_string();
    let cache: bool = match cache_str.parse() {
        Ok(c) => c,
        Err(_) => true,
    };

    let json_str = query
        .get("json")
        .unwrap_or(&String::new())
        .trim()
        .to_string();
    let json: bool = match json_str.parse() {
        Ok(j) => j,
        Err(_) => false,
    };

    let total_cache = crate::controller::score::get_data_for_scores_page(
        event_id,
        year,
        cache_map.get_ref(),
        cache,
    )
    .await;
    match total_cache {
        Ok(cache) => {
            if json {
                HttpResponse::Ok().json(cache)
            } else {
                let markup = crate::view::score::render_scores_template(&cache);
                HttpResponse::Ok()
                    .content_type("text/html")
                    .body(markup.into_string())
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": e.to_string()})),
    }
}

async fn admin(query: web::Query<HashMap<String, String>>) -> HttpResponse {
    router::router(query).await
}
