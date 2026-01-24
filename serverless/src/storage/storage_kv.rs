#![cfg(target_arch = "wasm32")]

use serde::{Deserialize, Serialize};

use super::storage_helpers::{parse_event_id, parse_year_from_end_date};
use super::storage_types::{AuthTokensDoc, EventDetailsDoc, EventListing};
use crate::storage::ServerlessStorage;
use rusty_golf_core::timed;
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

    pub fn kv_scores_cache_key(event_id: i32) -> String {
        format!("event:{event_id}:scores_cache")
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
        let timing = self.timing();
        let value = timed!(
            timing,
            "storage.kv_get_json_ms",
            self.kv
                .get(key)
                .json::<T>()
                .await
                .map_err(|e| StorageError::new(e.to_string()))
        )?;
        value.ok_or_else(|| StorageError::new(format!("KV key missing: {key}")))
    }

    pub async fn kv_get_optional_text(&self, key: &str) -> Result<Option<String>, StorageError> {
        let timing = self.timing();
        timed!(
            timing,
            "storage.kv_get_optional_fetch_ms",
            self.kv
                .get(key)
                .text()
                .await
                .map_err(|e| StorageError::new(e.to_string()))
        )
    }

    pub async fn kv_put_json<T>(&self, key: &str, value: &T) -> Result<(), StorageError>
    where
        T: Serialize + ?Sized,
    {
        let timing = self.timing();
        let payload = timed!(
            timing,
            "storage.kv_put_json_serialize_ms",
            serde_json::to_string(value).map_err(|e| StorageError::new(e.to_string()))
        )?;
        timed!(
            timing,
            "storage.kv_put_json_ms",
            self.kv
                .put(key, payload)
                .map_err(|e| StorageError::new(e.to_string()))?
                .execute()
                .await
                .map_err(|e| StorageError::new(e.to_string()))
        )?;
        Ok(())
    }

    pub async fn kv_put_json_with_ttl<T>(
        &self,
        key: &str,
        value: &T,
        ttl_seconds: u64,
    ) -> Result<(), StorageError>
    where
        T: Serialize + ?Sized,
    {
        let timing = self.timing();
        let payload = timed!(
            timing,
            "storage.kv_put_json_serialize_ms",
            serde_json::to_string(value).map_err(|e| StorageError::new(e.to_string()))
        )?;
        timed!(
            timing,
            "storage.kv_put_json_ttl_ms",
            self.kv
                .put(key, payload)
                .map_err(|e| StorageError::new(e.to_string()))?
                .expiration_ttl(ttl_seconds)
                .execute()
                .await
                .map_err(|e| StorageError::new(e.to_string()))
        )?;
        Ok(())
    }

    pub async fn r2_get_json<T>(&self, key: &str) -> Result<T, StorageError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let timing = self.timing();
        let obj = timed!(
            timing,
            "storage.r2_get_json_fetch_ms",
            self.bucket
                .get(key.to_string())
                .execute()
                .await
                .map_err(|e| StorageError::new(e.to_string()))
        )?;
        let obj = obj.ok_or_else(|| StorageError::new(format!("R2 key missing: {key}")))?;
        let body = obj
            .body()
            .ok_or_else(|| StorageError::new(format!("R2 body missing for key: {key}")))?;
        let text = timed!(
            timing,
            "storage.r2_get_json_body_ms",
            body.text().await.map_err(|e| StorageError::new(e.to_string()))
        )?;
        timed!(
            timing,
            "storage.r2_get_json_parse_ms",
            serde_json::from_str(&text).map_err(|e| StorageError::new(e.to_string()))
        )
    }

    pub async fn r2_put_json<T>(&self, key: &str, value: &T) -> Result<(), StorageError>
    where
        T: Serialize + ?Sized,
    {
        let timing = self.timing();
        let payload = timed!(
            timing,
            "storage.r2_put_json_serialize_ms",
            serde_json::to_string(value).map_err(|e| StorageError::new(e.to_string()))
        )?;
        timed!(
            timing,
            "storage.r2_put_json_ms",
            self.bucket
                .put(key.to_string(), payload)
                .execute()
                .await
                .map_err(|e| StorageError::new(e.to_string()))
        )?;
        Ok(())
    }

    pub async fn r2_list_keys_with_prefix(
        &self,
        prefix: Option<&str>,
    ) -> Result<Vec<String>, StorageError> {
        let timing = self.timing();
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
            let response = timed!(
                timing,
                "storage.r2_list_keys_page_ms",
                builder
                    .execute()
                    .await
                    .map_err(|e| StorageError::new(e.to_string()))
            )?;
            keys.extend(response.objects().into_iter().map(|obj| obj.key()));
            if !response.truncated() {
                break;
            }
            cursor = response.cursor();
        }
        Ok(keys)
    }

    pub async fn r2_key_exists(&self, key: &str) -> Result<bool, StorageError> {
        let timing = self.timing();
        let obj = timed!(
            timing,
            "storage.r2_key_exists_ms",
            self.bucket
                .get(key.to_string())
                .execute()
                .await
                .map_err(|e| StorageError::new(e.to_string()))
        )?;
        Ok(obj.is_some())
    }

    pub async fn kv_list_keys_with_prefix(
        &self,
        prefix: &str,
    ) -> Result<Vec<String>, StorageError> {
        let timing = self.timing();
        let mut keys = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let mut builder = self.kv.list().prefix(prefix.to_string());
            if let Some(cursor_value) = cursor {
                builder = builder.cursor(cursor_value);
            }
            let response = timed!(
                timing,
                "storage.kv_list_keys_page_ms",
                builder
                    .execute()
                    .await
                    .map_err(|e| StorageError::new(e.to_string()))
            )?;
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
            let year = parse_year_from_end_date(doc.end_date.as_deref());
            entries.push(EventListing {
                event_id,
                event_name: doc.event_name,
                year,
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
