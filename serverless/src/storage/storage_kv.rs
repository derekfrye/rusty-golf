#![cfg(target_arch = "wasm32")]

use serde::{Deserialize, Serialize};

use crate::storage::ServerlessStorage;
use rusty_golf_core::storage::StorageError;
use rusty_golf_core::timed;

impl ServerlessStorage {
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
}
