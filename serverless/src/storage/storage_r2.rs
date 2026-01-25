#![cfg(target_arch = "wasm32")]

use serde::Serialize;

use crate::storage::ServerlessStorage;
use rusty_golf_core::timed;
use rusty_golf_core::storage::StorageError;

impl ServerlessStorage {
    pub async fn r2_get_json<T>(&self, key: &str) -> Result<T, StorageError>
    where
        T: for<'de> serde::Deserialize<'de>,
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
}
