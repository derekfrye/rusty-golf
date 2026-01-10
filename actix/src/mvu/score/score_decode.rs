use rusty_golf_core::error::CoreError;
use rusty_golf_core::score::decode_score_request;
use rusty_golf_core::storage::Storage;
use std::collections::HashMap;
use std::hash::BuildHasher;

use crate::mvu::score::ScoreModel;

/// Parse query params into a `ScoreModel`, computing `cache_max_age` from event config.
///
/// # Errors
///
/// Returns `CoreError::Other` with human-readable messages for missing or invalid params.
pub async fn decode_request_to_model<S: BuildHasher>(
    query: &HashMap<String, String, S>,
    storage: &dyn Storage,
) -> Result<ScoreModel, CoreError> {
    let mut owned_query = HashMap::new();
    for (key, value) in query {
        owned_query.insert(key.clone(), value.clone());
    }
    decode_score_request(&owned_query, storage, |req, cache_max_age| {
        ScoreModel::new(
            req.event_id,
            req.year,
            req.use_cache,
            req.expanded,
            req.want_json,
            cache_max_age,
        )
    })
    .await
}
