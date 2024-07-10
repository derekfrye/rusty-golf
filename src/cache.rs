use crate::{Bettors, Scores};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize, Clone)]
pub struct Cache {
    pub data: Option<TotalCache>,
    pub cached_time: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TotalCache {
    pub bettor_struct: Vec<Bettors>,
    pub score_struct: Vec<Scores>,
    pub last_refresh: String,
}

pub type CacheMap = Arc<RwLock<HashMap<String, Cache>>>;
pub const CACHE_DURATION: chrono::Duration = chrono::Duration::minutes(5);

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

pub fn xya(cache: Cache) -> Result<TotalCache, Box<dyn std::error::Error>> {
    let cached_time = chrono::DateTime::parse_from_rfc3339(&cache.cached_time).unwrap();
    let cached_time_utc: DateTime<Utc> = cached_time.with_timezone(&Utc);
    let now = chrono::Utc::now();
    let elapsed = now - cached_time_utc;
    // if we're within the cache duration, return the cache
    if elapsed < CACHE_DURATION {
        if let Some(ref total_cache) = cache.data {
            let time_since = elapsed.num_seconds();
            let minutes = time_since / 60;
            let seconds = time_since % 60;
            let time_string = format!("{}m, {}s", minutes, seconds);
            let mut refreshed_cache = total_cache.clone();
            refreshed_cache.last_refresh = time_string;
            return Ok(refreshed_cache);
        }
    }
    Err("Cache expired".into())
}
