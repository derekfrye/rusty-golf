#![cfg(target_arch = "wasm32")]

use crate::storage::ServerlessStorage;
use rusty_golf_core::storage::StorageError;
use rusty_golf_core::timed;

use super::storage_helpers::{parse_event_id, parse_year_from_end_date};
use super::storage_types::{AuthTokensDoc, EventDetailsDoc, EventListing};

impl ServerlessStorage {
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
