use actix_web::web::{self, Data};
use actix_web::{HttpResponse, Responder};
use serde_json::json;
use sql_middleware::middleware::ConfigAndPool;
use std::collections::HashMap;

use crate::model::get_event_details;
use crate::mvu::score as mvu_score;
use crate::mvu::runtime::run_score;
use crate::view::score::{
    render_line_score_tables, render_summary_scores, scores_and_last_refresh_to_line_score_tables,
};
use crate::view::score::chart::render_drop_down_bar_pure;
use crate::view::score::types::RefreshData;

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

    // MVU: request-driven, no periodic triggers
    let mut model = mvu_score::ScoreModel::new(event_id, year, cache, expanded, json, cache_max_age);
    let _ = run_score(&mut model, mvu_score::Msg::PageLoad, mvu_score::Deps { config_and_pool: &config_and_pool }).await;

    if let Some(err) = model.error {
        return HttpResponse::InternalServerError().json(json!({"error": err}));
    }

    if model.want_json {
        if let Some(data) = model.data {
            HttpResponse::Ok().json(data)
        } else {
            HttpResponse::InternalServerError().json(json!({"error": "No data in model"}))
        }
    } else {
        if let Some(markup) = model.markup {
            HttpResponse::Ok().content_type("text/html").body(markup.into_string())
        } else {
            HttpResponse::InternalServerError().json(json!({"error": "No view produced"}))
        }
    }
}

#[allow(clippy::implicit_hasher)]
pub async fn scores_summary(
    query: web::Query<HashMap<String, String>>,
    abc: Data<ConfigAndPool>,
) -> impl Responder {
    let config_and_pool = abc.get_ref().clone();

    let event_id: i32 = match query.get("event").and_then(|s| s.trim().parse().ok()) {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest().json(json!({"error": "espn event parameter is required"}));
        }
    };
    let year: i32 = match query.get("yr").and_then(|s| s.trim().parse().ok()) {
        Some(y) => y,
        None => {
            return HttpResponse::BadRequest().json(json!({"error": "yr (year) parameter is required"}));
        }
    };
    let cache = !matches!(get_param_str(&query, "cache"), "0");
    let expanded = matches!(get_param_str(&query, "expanded"), "1");

    let cache_max_age: i64 = match get_event_details(&config_and_pool, event_id).await {
        Ok(event_details) => match event_details.refresh_from_espn { 1 => 99, _ => 0 },
        Err(_) => 0,
    };

    let mut model = mvu_score::ScoreModel::new(event_id, year, cache, expanded, false, cache_max_age);
    let _ = run_score(&mut model, mvu_score::Msg::PageLoad, mvu_score::Deps { config_and_pool: &config_and_pool }).await;

    if let Some(err) = model.error { return HttpResponse::InternalServerError().json(json!({"error": err})); }
    let Some(ref data) = model.data else { return HttpResponse::InternalServerError().json(json!({"error": "No data"})); };

    let summary = crate::controller::score::group_by_bettor_name_and_round(&data.score_struct);
    let markup = render_summary_scores(&summary);
    HttpResponse::Ok().content_type("text/html").body(markup.into_string())
}

#[allow(clippy::implicit_hasher)]
pub async fn scores_chart(
    query: web::Query<HashMap<String, String>>,
    abc: Data<ConfigAndPool>,
) -> impl Responder {
    let config_and_pool = abc.get_ref().clone();
    let event_id: i32 = match query.get("event").and_then(|s| s.trim().parse().ok()) { Some(id) => id, None => { return HttpResponse::BadRequest().json(json!({"error": "espn event parameter is required"})); } };
    let year: i32 = match query.get("yr").and_then(|s| s.trim().parse().ok()) { Some(y) => y, None => { return HttpResponse::BadRequest().json(json!({"error": "yr (year) parameter is required"})); } };
    let cache = !matches!(get_param_str(&query, "cache"), "0");
    let expanded = matches!(get_param_str(&query, "expanded"), "1");
    let cache_max_age: i64 = match get_event_details(&config_and_pool, event_id).await { Ok(ev) => match ev.refresh_from_espn { 1 => 99, _ => 0 }, Err(_) => 0 };

    let mut model = mvu_score::ScoreModel::new(event_id, year, cache, expanded, false, cache_max_age);
    let _ = run_score(&mut model, mvu_score::Msg::PageLoad, mvu_score::Deps { config_and_pool: &config_and_pool }).await;
    if let Some(err) = model.error { return HttpResponse::InternalServerError().json(json!({"error": err})); }
    let Some(ref data) = model.data else { return HttpResponse::InternalServerError().json(json!({"error": "No data"})); };
    let Some(global) = model.global_step_factor else { return HttpResponse::InternalServerError().json(json!({"error": "No global step factor"})); };
    let Some(ref factors) = model.player_step_factors else { return HttpResponse::InternalServerError().json(json!({"error": "No player step factors"})); };

    let summary_scores_x = crate::controller::score::group_by_bettor_name_and_round(&data.score_struct);
    let detailed_scores = crate::controller::score::group_by_bettor_golfer_round(&data.score_struct);
    let markup = render_drop_down_bar_pure(&summary_scores_x, &detailed_scores, global, factors);
    HttpResponse::Ok().content_type("text/html").body(markup.into_string())
}

#[allow(clippy::implicit_hasher)]
pub async fn scores_linescore(
    query: web::Query<HashMap<String, String>>,
    abc: Data<ConfigAndPool>,
) -> impl Responder {
    let config_and_pool = abc.get_ref().clone();
    let event_id: i32 = match query.get("event").and_then(|s| s.trim().parse().ok()) { Some(id) => id, None => { return HttpResponse::BadRequest().json(json!({"error": "espn event parameter is required"})); } };
    let year: i32 = match query.get("yr").and_then(|s| s.trim().parse().ok()) { Some(y) => y, None => { return HttpResponse::BadRequest().json(json!({"error": "yr (year) parameter is required"})); } };
    let cache = !matches!(get_param_str(&query, "cache"), "0");
    let expanded = matches!(get_param_str(&query, "expanded"), "1");
    let cache_max_age: i64 = match get_event_details(&config_and_pool, event_id).await { Ok(ev) => match ev.refresh_from_espn { 1 => 99, _ => 0 }, Err(_) => 0 };

    let mut model = mvu_score::ScoreModel::new(event_id, year, cache, expanded, false, cache_max_age);
    let _ = run_score(&mut model, mvu_score::Msg::PageLoad, mvu_score::Deps { config_and_pool: &config_and_pool }).await;
    if let Some(err) = model.error { return HttpResponse::InternalServerError().json(json!({"error": err})); }
    let Some(ref data) = model.data else { return HttpResponse::InternalServerError().json(json!({"error": "No data"})); };
    let Some(ref from_db) = model.from_db_scores else { return HttpResponse::InternalServerError().json(json!({"error": "No DB scores"})); };
    let bettor_struct = scores_and_last_refresh_to_line_score_tables(from_db);
    let refresh_data = RefreshData { last_refresh: data.last_refresh.clone(), last_refresh_source: data.last_refresh_source.clone() };
    let markup = render_line_score_tables(&bettor_struct, &refresh_data);
    HttpResponse::Ok().content_type("text/html").body(markup.into_string())
}
