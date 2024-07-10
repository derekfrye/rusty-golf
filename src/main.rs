mod espn;
mod db;
// mod scores;
mod cache;

use crate::cache::{CacheMap, TotalCache};

use actix_web::web::Data;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
// use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;


#[derive(Serialize, Deserialize, Clone)]
struct Bettors {
    bettor_name: String,
    total_score: i32,
    scoreboard_position_name: String,
    scoreboard_position: usize,
}

#[derive(Serialize, Deserialize, Clone)]
struct Scores {
    eup_id: i64,
    espn_id: i64,
    golfer_name: String,
    bettor_name: String,
    detailed_statistics: Statistic,
    group: i64,
}

#[derive(Serialize, Deserialize, Clone)]
struct Statistic {
    eup_id: i64,
    rounds: Vec<IntStat>,
    scores: Vec<IntStat>,
    tee_times: Vec<StringStat>,
    holes_completed: Vec<IntStat>,
    success_fail: ResultStatus,
    total_score: i32,
}

#[derive(Serialize, Deserialize, Clone)]
struct PlayerJsonResponse {
    data: Vec<HashMap<String, serde_json::Value>>,
    eup_ids: Vec<i64>,
}

#[derive(Serialize, Deserialize, Clone)]
struct StringStat {
    val: String,
    success: ResultStatus,
    last_refresh_date: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct IntStat {
    val: i32,
    success: ResultStatus,
    last_refresh_date: String,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
enum ResultStatus {
    NoData,
    NoDisplayValue,
    Success,
}

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

    let total_cache = get_data_for_scores_page(event_id, year, cache_map.get_ref(), cache).await;
    match total_cache {
        Ok(cache) => {
            if json {
                HttpResponse::Ok().json(cache)
            } else {
                // not impl yet
                HttpResponse::Ok().json(cache)
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": e.to_string()})),
    }
}

async fn get_data_for_scores_page(
    event_id: i32,
    year: i32,
    cache_map: &CacheMap,
    use_cache: bool,
) -> Result<TotalCache, Box<dyn std::error::Error>> {
    let cache = cache::get_or_create_cache(event_id, year, cache_map.clone()).await;

    if use_cache {

        return Ok(cache::xya(cache)?);

        // let cached_time = chrono::DateTime::parse_from_rfc3339(&cache.cached_time).unwrap();
        // let cached_time_utc: DateTime<Utc> = cached_time.with_timezone(&Utc);
        // let now = chrono::Utc::now();
        // let elapsed = now - cached_time_utc;
        // // if we're within the cache duration, return the cache
        // if elapsed < CACHE_DURATION {
        //     if let Some(ref total_cache) = cache.data {
        //         let time_since = elapsed.num_seconds();
        //         let minutes = time_since / 60;
        //         let seconds = time_since % 60;
        //         let time_string = format!("{}m, {}s", minutes, seconds);
        //         let mut refreshed_cache = total_cache.clone();
        //         refreshed_cache.last_refresh = time_string;
        //         return Ok(refreshed_cache);
        //     }
        // }
    }

    // reviewed, ok now for debugging
    let active_golfers = db::get_golfers_from_db(event_id).await?;
    let start_time = Instant::now();
    // reviewed, ok now for debugging
    let scores = espn::fetch_scores_from_espn(active_golfers.clone(), year, event_id).await;

    // ok
    let mut golfers_and_scores: Vec<Scores> = scores
        .iter()
        .map(|score| {
            let active_golfer = active_golfers
                .iter()
                .find(|g| g.eup_id == score.eup_id)
                .unwrap();
            Scores {
                eup_id: score.eup_id,
                golfer_name: active_golfer.golfer_name.clone(),
                detailed_statistics: score.clone(),
                bettor_name: active_golfer.bettor_name.clone(),
                group: active_golfer.group,
                espn_id: active_golfer.espn_id,
            }
        })
        .collect();

    // ok
    golfers_and_scores.sort_by(|a, b| {
        if a.group == b.group {
            a.eup_id.cmp(&b.eup_id)
        } else {
            a.group.cmp(&b.group)
        }
    });

    // ok
    let mut totals: HashMap<String, i32> = HashMap::new();
    for golfer in &golfers_and_scores {
        *totals.entry(golfer.bettor_name.clone()).or_insert(0) +=
            golfer.detailed_statistics.total_score;
    }

    let mut bettors: Vec<Bettors> = totals
        .into_iter()
        .map(|(name, total)| Bettors {
            bettor_name: name,
            total_score: total,
            scoreboard_position_name: String::new(),
            scoreboard_position: 0,
        })
        .collect();

    bettors.sort_by(|a, b| a.total_score.cmp(&b.total_score));

    for (i, bettor) in bettors.iter_mut().enumerate() {
        bettor.scoreboard_position = i;
        bettor.scoreboard_position_name = match i {
            0 => "TOP GOLFER".to_string(),
            1 => "FIRST LOSER".to_string(),
            2 => "MEH".to_string(),
            3 => "SEEN BETTER DAYS".to_string(),
            4 => "NOT A CHANCE".to_string(),
            _ => "WORST OF THE WORST".to_string(),
        }
    }

    let time_since = start_time.elapsed();
    let minutes = time_since.as_secs() / 60;
    let seconds = time_since.as_secs() % 60;
    let time_string = format!("{}m, {}s", minutes, seconds);

    let total_cache = TotalCache {
        bettor_struct: bettors,
        score_struct: golfers_and_scores,
        last_refresh: time_string,
    };

    let key = format!("{}{}", event_id, year);
    let mut cache = cache_map.write().await;
    cache.insert(
        key,
       crate::cache::Cache {
            data: Some(total_cache.clone()),
            cached_time: chrono::Utc::now().to_rfc3339(),
        },
    );

    Ok(total_cache)
}