use crate::args::CleanArgs;
use actix_web::web::{self, Data};
use actix_web::{HttpResponse, Responder};
use serde_json::json;
use sql_middleware::middleware::ConfigAndPool;
// use sqlx_middleware::db::{ConfigAndPool as ConfigAndPoolOld, DatabaseType,};
use crate::controller::espn::fetch_scores_from_espn;
use crate::model::{self, format_time_ago_for_score_view, DetailedScore, SummaryDetailedScores};
use crate::model::{AllBettorScoresByRound, BettorScoreByRound, Bettors, ScoreData, Scores};
use crate::view::score::render_scores_template;
use std::collections::{BTreeMap, HashMap};

pub async fn scores(
    query: web::Query<HashMap<String, String>>,
    abc: Data<ConfigAndPool>,
    args: Data<CleanArgs>,
) -> impl Responder {
    // let db = Db::new(abc.get_ref().clone()).unwrap();
    let config_and_pool = abc.get_ref().clone();
    let clean_args = args.get_ref().clone();
    // let pool = abc.get_ref().clone().pool.get().await.unwrap();
    // let conn = MiddlewarePool::get_connection(pool).await.unwrap();

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

    // Parse cache_max_age parameter with better error handling and default values
    let cache_max_age: i64 = query
        .get("cache_max_age")
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or_else(|| {
            // Use the configured value if available, otherwise default to 0
            clean_args.dont_poll_espn_after_num_days.unwrap_or(0)
        });

    // let mut cfg = deadpool_postgres::Config::new();
    // let dbcn: ConfigAndPoolOld;
    // cfg.dbname = Some("xxx".to_string());
    // dbcn = ConfigAndPoolOld::new(cfg, DatabaseType::Sqlite).await;
    // let db = Db::new(dbcn.clone()).unwrap();

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

    // let start_time = Instant::now();
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

    // let time_since = start_time.elapsed();
    // let minutes = time_since.as_secs() / 60;
    // let seconds = time_since.as_secs() % 60;
    // let time_string = format!("{}m, {}s", minutes, seconds);

    let x = chrono::Utc::now().naive_utc() - golfers_and_scores.last_refresh;

    let total_cache = ScoreData {
        bettor_struct: bettors,
        score_struct: golfers_and_scores.score_struct,
        last_refresh: format_time_ago_for_score_view(x),
        last_refresh_source: golfers_and_scores.last_refresh_source,
    };

    // let key = format!("{}{}", event_id, year);
    // let mut cache = cache_map.write().await;
    // cache.insert(
    //     key,
    //     Cache {
    //         data: Some(total_cache.clone()),
    //         cached_time: chrono::Utc::now().to_rfc3339(),
    //     },
    // );

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
    // key = bettor, value = hashmap of rounds and the corresponding score
    let mut rounds_by_bettor_storing_score_val: HashMap<String, Vec<(isize, isize)>> =
        HashMap::new();

    // Accumulate scores by bettor and round
    for score in scores {
        let bettor_name = &score.bettor_name;

        // for debug watching
        // let golfers_name = &score.golfer_name;
        // let _ = golfers_name.len();

        for (round_idx, round_score) in score.detailed_statistics.round_scores.iter().enumerate() {
            // Get or create the vector of scores for this bettor without cloning
            let a_single_bettors_scores = rounds_by_bettor_storing_score_val
                .entry(bettor_name.to_string())
                .or_default();
            // Convert round_idx to isize safely, defaulting to 0 if conversion fails
            let round_idx_isize = match isize::try_from(round_idx) {
                Ok(val) => val,
                Err(_) => {
                    eprintln!("Warning: Failed to convert round index {} to isize", round_idx);
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

    // Preserves order of bettors
    for score in scores {
        let bettor_name = &score.bettor_name;
        if rounds_by_bettor_storing_score_val.contains_key(bettor_name)
            && !bettor_names.contains(bettor_name)
        {
            bettor_names.push(bettor_name.clone());
        }
    }

    // Preserves order of bettors
    // this actually just needs to sum all the scores where the rounds are 0, store that val, sum all scores where rounds are 1, store that value, etc
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
    // Nested HashMap: bettor -> golfer -> round -> accumulated score
    let mut scores_map: HashMap<String, HashMap<String, BTreeMap<i32, i32>>> = HashMap::new();

    // To preserve the order of bettors and golfers as they appear in the input
    let mut bettor_order: Vec<String> = Vec::new();
    let mut golfer_order_map: HashMap<String, Vec<String>> = HashMap::new();

    // Accumulate scores by bettor, golfer, and round
    for score in scores {
        let bettor_name = &score.bettor_name;
        let golfer_name = &score.golfer_name;

        // Track the order of bettors
        if !bettor_order.contains(bettor_name) {
            bettor_order.push(bettor_name.clone());
        }

        // Track the order of golfers per bettor
        golfer_order_map
            .entry(bettor_name.clone())
            .or_default()
            .push(golfer_name.clone());

        for (round_idx, score) in score.detailed_statistics.round_scores.iter().enumerate() {
            let round_val = (round_idx as i32) + 1; // Assuming rounds start at 1
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

    // Remove duplicate golfers while preserving order
    for golfers in golfer_order_map.values_mut() {
        let mut seen = HashMap::new();
        golfers.retain(|golfer| seen.insert(golfer.clone(), ()).is_none());
    }

    // Build the summary scores
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

                        let (round_numbers, scores) = rounds.iter().cloned().unzip();

                        summary_scores.detailed_scores.push(DetailedScore {
                            bettor_name: bettor_name.clone(),
                            golfer_name: golfer_name.clone(),
                            rounds: round_numbers,
                            scores,
                        });
                    }
                }
            }
        }
    }

    summary_scores
}
