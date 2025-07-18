use actix_web::web::{self, Data};
use actix_web::{HttpResponse, Responder};
use serde_json::json;
use sql_middleware::middleware::ConfigAndPool;
// use sqlx_middleware::db::{ConfigAndPool as ConfigAndPoolOld, DatabaseType,};
use crate::controller::espn::fetch_scores_from_espn;
// Import the renamed function
use crate::model::{self, get_event_details, DetailedScore, SummaryDetailedScores, format_time_ago_for_score_view};
use crate::model::{AllBettorScoresByRound, BettorScoreByRound, Bettors, ScoreData, Scores};
use crate::view::score::render_scores_template;
use std::collections::{BTreeMap, HashMap};

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

    // Helper function to get a query parameter with a default value
    fn get_param_str<'a>(query: &'a HashMap<String, String>, key: &str) -> &'a str {
        query.get(key).map(|s| s.as_str()).unwrap_or("")
    }

    // Parse the boolean parameters
    let cache = match get_param_str(&query, "cache") {
        "1" => true,
        "0" => false,
        _ => true, // Default to true
    };

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
            1 => 99,  // Refresh from ESPN requested, set cache age to 99 (which means only read from db once 99 days has passed)
            0 => 0, // Do not refresh from ESPN, set cache age to 0 (which menas always read from db)
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

pub async fn get_data_for_scores_page(
    event_id: i32,
    year: i32,
    use_cache: bool,
    config_and_pool: &ConfigAndPool,
    cache_max_age: i64,
) -> Result<ScoreData, Box<dyn std::error::Error>> {
    let active_golfers = model::get_golfers_from_db(config_and_pool, event_id).await?;

    let golfers_and_scores = fetch_scores_from_espn(
        active_golfers.clone(),
        year,
        event_id,
        config_and_pool,
        use_cache,
        cache_max_age,
    )
    .await?;

    let mut totals: HashMap<String, i32> = HashMap::new();
    for golfer in &golfers_and_scores.score_struct {
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

    bettors.sort_by(|a, b| {
        a.total_score
            .cmp(&b.total_score)
            .then_with(|| a.bettor_name.cmp(&b.bettor_name))
    });

    for (i, bettor) in bettors.iter_mut().enumerate() {
        bettor.scoreboard_position = i;
        bettor.scoreboard_position_name = match i {
            0 => "TOP GOLFER".to_string(),
            1 => "FIRST LOSER".to_string(),
            2 => "MEH".to_string(),
            3 => "SEEN BETTER DAYS".to_string(),
            4 => "NOT A CHANCE".to_string(),
            _ => "WORST OF THE WORST".to_string(),
        };
    }

    let x = chrono::Utc::now().naive_utc() - golfers_and_scores.last_refresh;

    let total_cache = ScoreData {
        bettor_struct: bettors,
        score_struct: golfers_and_scores.score_struct,
        last_refresh: format_time_ago_for_score_view(x),
        last_refresh_source: golfers_and_scores.last_refresh_source,
    };

    Ok(total_cache)
}

pub fn group_by_scores(scores: Vec<Scores>) -> Vec<(usize, Vec<Scores>)> {
    let mut grouped_scores: HashMap<usize, Vec<Scores>> = HashMap::new();

    for score in scores {
        grouped_scores
            .entry(score.group as usize)
            .or_default()
            .push(score);
    }

    sort_scores(grouped_scores)
}

fn sort_scores(grouped_scores: HashMap<usize, Vec<Scores>>) -> Vec<(usize, Vec<Scores>)> {
    let mut sorted_scores: Vec<(usize, Vec<Scores>)> = grouped_scores.into_iter().collect();

    sorted_scores.sort_by_key(|(group, _)| *group); // Sort by the `group` key

    sorted_scores
}

pub fn group_by_bettor_name_and_round(scores: &[Scores]) -> AllBettorScoresByRound {
    let mut rounds_by_bettor_storing_score_val: HashMap<String, Vec<(isize, isize)>> =
        HashMap::new();

    for score in scores {
        let bettor_name = &score.bettor_name;

        for (round_idx, round_score) in score.detailed_statistics.round_scores.iter().enumerate() {
            let a_single_bettors_scores = rounds_by_bettor_storing_score_val
                .entry(bettor_name.to_string())
                .or_default();
            let round_idx_isize = match isize::try_from(round_idx) {
                Ok(val) => val,
                Err(_) => {
                    eprintln!(
                        "Warning: Failed to convert round index {round_idx} to isize"
                    );
                    0
                }
            };
            a_single_bettors_scores.push((round_idx_isize, round_score.val as isize));
        }
    }

    let mut summary_scores = AllBettorScoresByRound {
        summary_scores: Vec::new(),
    };
    let mut bettor_names: Vec<String> = Vec::new();

    for score in scores {
        let bettor_name = &score.bettor_name;
        if rounds_by_bettor_storing_score_val.contains_key(bettor_name)
            && !bettor_names.contains(bettor_name)
        {
            bettor_names.push(bettor_name.clone());
        }
    }

    for bettor_name in &bettor_names {
        if rounds_by_bettor_storing_score_val.contains_key(bettor_name) {
            let res1 = rounds_by_bettor_storing_score_val
                .get(bettor_name)
                .unwrap()
                .iter();

            let result = res1
                .fold(BTreeMap::new(), |mut acc, &(k, v)| {
                    *acc.entry(k).or_insert(0) += v;
                    acc
                })
                .into_iter()
                .collect::<Vec<(isize, isize)>>();

            let (computed_rounds, new_scores): (Vec<isize>, Vec<isize>) =
                result.iter().cloned().unzip();

            summary_scores.summary_scores.push(BettorScoreByRound {
                bettor_name: bettor_name.clone(),
                computed_rounds,
                scores_aggregated_by_golf_grp_by_rd: new_scores,
            });
        }
    }

    summary_scores
}

pub fn group_by_bettor_golfer_round(scores: &Vec<Scores>) -> SummaryDetailedScores {
    let mut scores_map: HashMap<String, HashMap<String, BTreeMap<i32, i32>>> = HashMap::new();
    
    let mut espn_id_map: HashMap<(String, String), i64> = HashMap::new();

    let mut bettor_order: Vec<String> = Vec::new();
    let mut golfer_order_map: HashMap<String, Vec<String>> = HashMap::new();

    for score in scores {
        let bettor_name = &score.bettor_name;
        let golfer_name = &score.golfer_name;

        if !bettor_order.contains(bettor_name) {
            bettor_order.push(bettor_name.clone());
        }

        golfer_order_map
            .entry(bettor_name.clone())
            .or_default()
            .push(golfer_name.clone());
            
        espn_id_map.insert((bettor_name.clone(), golfer_name.clone()), score.espn_id);

        for (round_idx, score) in score.detailed_statistics.round_scores.iter().enumerate() {
            let round_val = (round_idx as i32) + 1;
            let round_score = score.val;

            scores_map
                .entry(bettor_name.clone())
                .or_default()
                .entry(golfer_name.clone())
                .or_default()
                .entry(round_val)
                .and_modify(|e| {
                    *e += round_score;
                })
                .or_insert(round_score);
        }
    }

    for golfers in golfer_order_map.values_mut() {
        let mut seen = HashMap::new();
        golfers.retain(|golfer| seen.insert(golfer.clone(), ()).is_none());
    }

    let mut summary_scores = SummaryDetailedScores {
        detailed_scores: Vec::new(),
    };

    for bettor_name in bettor_order {
        if let Some(golfers_map) = scores_map.get(&bettor_name) {
            if let Some(golfers_ordered) = golfer_order_map.get(&bettor_name) {
                for golfer_name in golfers_ordered {
                    if let Some(rounds_map) = golfers_map.get(golfer_name) {
                        let mut rounds: Vec<(i32, i32)> =
                            rounds_map.iter().map(|(&k, &v)| (k, v)).collect();
                        rounds.sort_by_key(|&(round, _)| round);

                        let (round_numbers, round_scores): (Vec<i32>, Vec<i32>) = rounds.iter().cloned().unzip();

                        let golfer_espn_id = espn_id_map
                            .get(&(bettor_name.clone(), golfer_name.clone()))
                            .copied()
                            .unwrap_or(0);
                        
                        summary_scores.detailed_scores.push(DetailedScore {
                            bettor_name: bettor_name.clone(),
                            golfer_name: golfer_name.clone(),
                            golfer_espn_id,
                            rounds: round_numbers,
                            scores: round_scores,
                        });
                    }
                }
            }
        }
    }

    summary_scores
}
