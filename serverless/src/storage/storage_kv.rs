#![cfg(target_arch = "wasm32")]

use serde::{Deserialize, Serialize};

use super::storage_helpers::parse_event_id;
use super::storage_types::{AuthTokensDoc, EventDetailsDoc, EventListing};
use crate::storage::ServerlessStorage;
use rusty_golf_core::storage::StorageError;

impl ServerlessStorage {
    pub fn scores_key(event_id: i32) -> String {
        format!("events/{event_id}/scores.json")
    }

    pub fn espn_cache_key(event_id: i32) -> String {
        format!("cache/espn/{event_id}.json")
    }

    pub fn kv_event_details_key(event_id: i32) -> String {
        format!("event:{event_id}:details")
    }

    pub fn kv_golfers_key(event_id: i32) -> String {
        format!("event:{event_id}:golfers")
    }

    pub fn kv_player_factors_key(event_id: i32) -> String {
        format!("event:{event_id}:player_factors")
    }

    pub fn kv_last_refresh_key(event_id: i32) -> String {
        format!("event:{event_id}:last_refresh")
    }

    pub fn kv_seeded_at_key(event_id: i32, suffix: &str) -> String {
        format!("event:{event_id}:{suffix}:seeded_at")
    }

    pub fn kv_force_espn_fail_key(event_id: i32) -> String {
        format!("event:{event_id}:force_espn_fail")
    }

    pub fn kv_test_lock_key(event_id: i32) -> String {
        format!("event:{event_id}:test_lock")
    }

    pub fn kv_test_lock_prefix() -> &'static str {
        "event:"
    }

    pub async fn kv_get_json<T>(&self, key: &str) -> Result<T, StorageError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let value = self
            .kv
            .get(key)
            .json::<T>()
            .await
            .map_err(|e| StorageError::new(e.to_string()))?;
        value.ok_or_else(|| StorageError::new(format!("KV key missing: {key}")))
    }

    pub async fn kv_put_json<T>(&self, key: &str, value: &T) -> Result<(), StorageError>
    where
        T: Serialize + ?Sized,
    {
        let payload = serde_json::to_string(value).map_err(|e| StorageError::new(e.to_string()))?;
        self.kv
            .put(key, payload)
            .map_err(|e| StorageError::new(e.to_string()))?
            .execute()
            .await
            .map_err(|e| StorageError::new(e.to_string()))?;
        Ok(())
    }

    pub async fn r2_get_json<T>(&self, key: &str) -> Result<T, StorageError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let obj = self
            .bucket
            .get(key.to_string())
            .execute()
            .await
            .map_err(|e| StorageError::new(e.to_string()))?;
        let obj = obj.ok_or_else(|| StorageError::new(format!("R2 key missing: {key}")))?;
        let body = obj
            .body()
            .ok_or_else(|| StorageError::new(format!("R2 body missing for key: {key}")))?;
        let text = body
            .text()
            .await
            .map_err(|e| StorageError::new(e.to_string()))?;
        serde_json::from_str(&text).map_err(|e| StorageError::new(e.to_string()))
    }

    pub async fn r2_put_json<T>(&self, key: &str, value: &T) -> Result<(), StorageError>
    where
        T: Serialize + ?Sized,
    {
        let payload = serde_json::to_string(value).map_err(|e| StorageError::new(e.to_string()))?;
        self.bucket
            .put(key.to_string(), payload)
            .execute()
            .await
            .map_err(|e| StorageError::new(e.to_string()))?;
        Ok(())
    }

    pub async fn r2_list_keys_with_prefix(
        &self,
        prefix: Option<&str>,
    ) -> Result<Vec<String>, StorageError> {
        let mut keys = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let mut builder = self.bucket.list();
            if let Some(prefix_value) = prefix {
                if !prefix_value.is_empty() {
                    builder = builder.prefix(prefix_value.to_string());
                }
            }
            if let Some(cursor_value) = cursor {
                builder = builder.cursor(cursor_value);
            }
            let response = builder
                .execute()
                .await
                .map_err(|e| StorageError::new(e.to_string()))?;
            keys.extend(response.objects().into_iter().map(|obj| obj.key()));
            if !response.truncated() {
                break;
            }
            cursor = response.cursor();
        }
        Ok(keys)
    }

    pub async fn kv_list_keys_with_prefix(
        &self,
        prefix: &str,
    ) -> Result<Vec<String>, StorageError> {
        let mut keys = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let mut builder = self.kv.list().prefix(prefix.to_string());
            if let Some(cursor_value) = cursor {
                builder = builder.cursor(cursor_value);
            }
            let response = builder
                .execute()
                .await
                .map_err(|e| StorageError::new(e.to_string()))?;
            keys.extend(response.keys.into_iter().map(|key| key.name));
            if response.list_complete {
                break;
            }
            cursor = response.cursor;
        }
        Ok(keys)
    }

    pub async fn list_event_listings(&self) -> Result<Vec<EventListing>, StorageError> {
        let keys = self.kv_list_keys_with_prefix("event:").await?;
        let mut entries = Vec::new();
        for key in keys {
            let event_id = match parse_event_id(&key, ":details") {
                Some(value) => value,
                None => continue,
            };
            let doc: EventDetailsDoc = self.kv_get_json(&key).await?;
            entries.push(EventListing {
                event_id,
                event_name: doc.event_name,
                score_view_step_factor: doc.score_view_step_factor,
                refresh_from_espn: doc.refresh_from_espn,
            });
        }
        entries.sort_by_key(|entry| entry.event_id);
        Ok(entries)
    }

    pub async fn auth_token_valid(&self, token: &str) -> Result<bool, StorageError> {
        let keys = self.kv_list_keys_with_prefix("event:").await?;
        for key in keys {
            if !key.ends_with(":auth_tokens") {
                continue;
            }
            let doc: AuthTokensDoc = match self.kv_get_json(&key).await {
                Ok(value) => value,
                Err(_) => continue,
            };
            if doc.tokens.iter().any(|stored| stored == token) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
