mod model {
    pub mod db;
    pub mod model;
    pub mod admin_model;
}
mod controller {
    pub mod cache;
    pub mod espn;
    pub mod score;
    pub mod templates {
        pub mod admin {
            pub mod admin;
            pub mod admin00;
        }
        pub mod index;
        pub mod score;
    }
}

use model::admin_model::AlphaNum14;
use model::db;
use model::model::CacheMap;

use actix_web::web::Data;
use actix_web::{ web, App, HttpResponse, HttpServer, Responder };
// use chrono::{DateTime, Utc};
use actix_files::Files;
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use tokio::sync::RwLock;

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
        .run().await
}

async fn index(query: web::Query<HashMap<String, String>>) -> impl Responder {
    let event_str = query.get("event").unwrap_or(&String::new()).trim().to_string();
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

    let markup = crate::controller::templates::index::render_index_template(title);
    HttpResponse::Ok().content_type("text/html").body(markup.into_string())
}

async fn scores(
    cache_map: Data<CacheMap>,
    query: web::Query<HashMap<String, String>>
) -> impl Responder {
    let event_str = query.get("event").unwrap_or(&String::new()).trim().to_string();
    let event_id: i32 = match event_str.parse() {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(
                json!({"error": "espn event parameter is required"})
            );
        }
    };

    let year_str = query.get("yr").unwrap_or(&String::new()).trim().to_string();
    let year: i32 = match year_str.parse() {
        Ok(y) => y,
        Err(_) => {
            return HttpResponse::BadRequest().json(
                json!({"error": "yr (year) parameter is required"})
            );
        }
    };

    let cache_str = query.get("cache").unwrap_or(&String::new()).trim().to_string();
    let cache: bool = match cache_str.parse() {
        Ok(c) => c,
        Err(_) => true,
    };

    let json_str = query.get("json").unwrap_or(&String::new()).trim().to_string();
    let json: bool = match json_str.parse() {
        Ok(j) => j,
        Err(_) => false,
    };

    let total_cache = crate::controller::score::get_data_for_scores_page(
        event_id,
        year,
        cache_map.get_ref(),
        cache
    ).await;
    match total_cache {
        Ok(cache) => {
            if json {
                HttpResponse::Ok().json(cache)
            } else {
                let markup = crate::controller::templates::score::render_scores_template(&cache);
                HttpResponse::Ok().content_type("text/html").body(markup.into_string())
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": e.to_string()})),
    }
}

async fn admin(query: web::Query<HashMap<String, String>>) -> HttpResponse {
    let token_str = query.get("token").unwrap_or(&String::new()).trim().to_string();

    // let mut token: AlphaNum14 = AlphaNum14::default();
    let token: AlphaNum14 = match AlphaNum14::parse(&token_str) {
        Ok(id) => id,
        Err(_) => AlphaNum14::default(),
    };

    let data = query.get("data").unwrap_or(&String::new()).trim().to_string();
    let missing_tables = query
        .get("admin00_missing_tables")
        .unwrap_or(&String::new())
        .trim()
        .to_string();
    let times_run = query.get("times_run").unwrap_or(&String::new()).trim().to_string();

    let player_vec = vec![
        model::admin_model::Player {
            id: 1,
            name: "Player1".to_string(),
        },
        model::admin_model::Player {
            id: 2,
            name: "Player2".to_string(),
        }
    ];
    let bettor_vec = vec![
        model::admin_model::Bettor {
            uid: 1,
            name: "Bettor1".to_string(),
        },
        model::admin_model::Bettor {
            uid: 2,
            name: "Bettor2".to_string(),
        }
    ];

    let body =
        r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>Unauthorized</title>
        <style>
            body { font-family: Arial, sans-serif; background-color: #f4f4f4; text-align: center; padding: 50px; }
            h1 { color: #333; }
            p { color: #666; }
        </style>
    </head>
    <body>
        <h1>401 Unauthorized</h1>
    </body>
    </html>
    "#;

    // check query string token against the token this container was started with
    match env::var("TOKEN") {
        Ok(env_token) => {
            if env_token != token.value() {
                return HttpResponse::Unauthorized()
                    .content_type("text/html; charset=utf-8")
                    .body(body);
            } else {
                // authorized
                if !missing_tables.is_empty() {
                    let markup_from_admin =
                        crate::controller::templates::admin::admin00::create_tables(
                            missing_tables,
                            times_run,
                        ).await;

                        // dbg!("markup_from_admin", &markup_from_admin.times_run_int);

                        let header = json!({
                            "reenablebutton": "1",
                            "times_run": markup_from_admin.times_run_int
                        });

                    HttpResponse::Ok()
                        .content_type("text/html")
                        .insert_header(("HX-Trigger", header.to_string())) // Add the HX-Trigger header, this tells the create table button to reenable (based on a fn in admin.js)
                        .body(markup_from_admin.html.into_string())
                } else if data.is_empty() {
                    let markup = crate::controller::templates::admin::admin::render_default_page(
                        &player_vec,
                        &bettor_vec
                    ).await;

                    HttpResponse::Ok().content_type("text/html").body(markup.into_string())
                } else {
                    let markup_from_admin =
                        crate::controller::templates::admin::admin::display_received_data(
                            player_vec,
                            bettor_vec,
                            data
                        );
                    HttpResponse::Ok()
                        .content_type("text/html")
                        .body(markup_from_admin.into_string())
                }
            }
        }
        Err(_) => {
            return HttpResponse::Unauthorized().content_type("text/html; charset=utf-8").body(body);
        }
    }
}
