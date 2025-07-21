use actix_web::web::{self, Data};
use actix_web::{HttpResponse, Responder};
use serde_json::json;
use sql_middleware::middleware::ConfigAndPool;
use std::collections::HashMap;

use super::data_service::get_data_for_scores_page;
use crate::model::get_event_details;
use crate::view::score::render_scores_template;

// Helper function to get a query parameter with a default value
fn get_param_str<'a>(query: &'a HashMap<String, String>, key: &str) -> &'a str {
    query.get(key).map_or("", |s| s.as_str())
}

// The `implicit_hasher` lint is allowed here because the `HashMap` is created by `actix-web`
// as part of the query string parsing. We cannot control the hasher used in this case,
// and the performance impact is negligible for a small number of query parameters.
#[allow(clippy::implicit_hasher)]
pub async fn scores(
    query: web::Query<HashMap<String, String>>,
    abc: Data<ConfigAndPool>,
) -> impl Responder {
    let config_and_pool = abc.get_ref().clone();

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

    // Parse the boolean parameters
    let cache = !matches!(get_param_str(&query, "cache"), "0");

    let json = match get_param_str(&query, "json") {
        "1" => true,
        "0" => false,
        other => other.parse().unwrap_or(false), // Default to false
    };

    let expanded = match get_param_str(&query, "expanded") {
        "1" => true,
        "0" => false,
        other => other.parse().unwrap_or(false), // Default to false
    };

    // Determine cache_max_age based on refresh_from_espn flag from the database
    let cache_max_age: i64 = match get_event_details(&config_and_pool, event_id).await {
        Ok(event_details) => match event_details.refresh_from_espn {
            1 => 99, // Refresh from ESPN requested, set cache age to 99 (which means only read from db once 99 days has passed)
            _ => 0,  // Any other value, default to not refreshing (cache age 0)
        },
        Err(_) => 0, // If error fetching details, default to not refreshing (cache age 0)
    };

    let total_cache =
        get_data_for_scores_page(event_id, year, cache, &config_and_pool, cache_max_age).await;

    match total_cache {
        Ok(cache) => {
            if json {
                HttpResponse::Ok().json(cache)
            } else {
                let markup =
                    render_scores_template(&cache, expanded, &config_and_pool, event_id).await;
                match markup {
                    Ok(markup) => HttpResponse::Ok()
                        .content_type("text/html")
                        .body(markup.into_string()),
                    Err(e) => {
                        HttpResponse::InternalServerError().json(json!({"error": e.to_string()}))
                    }
                }
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": e.to_string()})),
    }
}
