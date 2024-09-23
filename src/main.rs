mod db;
mod espn;
// mod scores;
mod cache;
mod score;
mod model;
mod templates {
    pub mod scores;
}

use crate::model::CacheMap;

use actix_web::web::Data;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
// use chrono::{DateTime, Utc};

use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    // for (key, value) in std::env::vars() {
    //     println!("{}: {}", key, value);
    // }

    // // now print working directory
    // let cwd = std::env::current_dir().unwrap();
    // println!("Current working directory: {}", cwd.display());

    let cache_map: CacheMap = Arc::new(RwLock::new(HashMap::new()));

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(cache_map.clone()))
            .route("/", web::get().to(index))
            .route("/scores", web::get().to(scores))
    })
    .bind("0.0.0.0:8083")?
    .run()
    .await
}

// async fn group_by_scores(scores: Vec<Scores>) -> HashMap<i32, Vec<Scores>> {
//     let mut grouped_scores = HashMap::new();
//     for score in scores {
//         grouped_scores
//             .entry(score.group)
//             .or_insert(Vec::new())
//             .push(score);
//     }
//     grouped_scores
// }

// async fn seq(count: usize) -> Vec<usize> {
//     (0..count).collect()
// }

async fn index() -> impl Responder {
    HttpResponse::Ok().body("Index")
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
                .json(json!({"error": "espn event parameter is required"}))
        }
    };

    let year_str = query.get("yr").unwrap_or(&String::new()).trim().to_string();
    let year: i32 = match year_str.parse() {
        Ok(y) => y,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": "yr (year) parameter is required"}))
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

    let total_cache =
        crate::score::get_data_for_scores_page(event_id, year, cache_map.get_ref(), cache).await;
    match total_cache {
        Ok(cache) => {
            if json {
                HttpResponse::Ok().json(cache)
            } else {
                let markup = crate::templates::scores::render_scores_template(&cache);
                HttpResponse::Ok().content_type("text/html").body(markup.into_string())
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": e.to_string()})),
    }
}
