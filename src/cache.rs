use crate::model::{Cache, CacheMap, ScoreData};
use chrono::{DateTime, Utc};
// use serde::{Deserialize, Serialize};

const CACHE_DURATION: chrono::Duration = chrono::Duration::minutes(5);

pub async fn get_or_create_cache(event: i32, year: i32, cache_map: CacheMap) -> Cache {
    let key = format!("{}{}", event, year);
    let mut map = cache_map.write().await;
    if let Some(cache) = map.get(&key) {
        return cache.clone();
    }

    let new_cache = Cache {
        data: None,
        cached_time: chrono::Utc::now().to_rfc3339(),
    };
    map.insert(key.clone(), new_cache.clone());
    new_cache
}

pub fn xya(cache: Cache) -> Result<ScoreData, Box<dyn std::error::Error>> {
    let cached_time = chrono::DateTime::parse_from_rfc3339(&cache.cached_time).unwrap();
    let cached_time_utc: DateTime<Utc> = cached_time.with_timezone(&Utc);
    let now = chrono::Utc::now();
    let cache_age = now - cached_time_utc;
    // if we're within the cache duration, return the cache
    if cache_age < CACHE_DURATION {
        if let Some(ref score_data) = cache.data {
            let time_since = cache_age.num_seconds();
            let minutes = time_since / 60;
            let seconds = time_since % 60;
            let time_string = format!("{}m, {}s", minutes, seconds);
            let mut refreshed_cache = score_data.clone();
            refreshed_cache.last_refresh = time_string;
            return Ok(refreshed_cache);
        }
    }
    Err("Cache expired".into())
}
