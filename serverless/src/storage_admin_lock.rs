#![cfg(target_arch = "wasm32")]

use chrono::Utc;
use rusty_golf_core::storage::StorageError;
use std::collections::HashMap;

use crate::storage::ServerlessStorage;
use crate::storage_helpers::{format_rfc3339, parse_rfc3339};
use crate::storage_types::{TestLockDoc, TestLockMode};

impl ServerlessStorage {
    pub async fn admin_test_lock(
        &self,
        event_id: i32,
        token: &str,
        ttl_secs: i64,
        mode: TestLockMode,
        force: bool,
    ) -> Result<(bool, bool), StorageError> {
        let key = Self::kv_test_lock_key(event_id);
        let mut doc: TestLockDoc = match self.kv_get_json(&key).await {
            Ok(value) => value,
            Err(_) => TestLockDoc {
                shared_holders: HashMap::new(),
                exclusive_holder: None,
            },
        };

        let now = Utc::now().naive_utc();
        doc.shared_holders.retain(|_, expires_at| {
            parse_rfc3339(expires_at)
                .map(|ts| ts > now)
                .unwrap_or(false)
        });
        if let Some((_, expires_at)) = doc.exclusive_holder.clone() {
            let expired = parse_rfc3339(&expires_at)
                .map(|ts| ts <= now)
                .unwrap_or(true);
            if expired {
                doc.exclusive_holder = None;
            }
        }

        if force {
            doc.shared_holders.clear();
            doc.exclusive_holder = None;
        }

        let expires_at = now + chrono::Duration::seconds(ttl_secs.max(1));
        let expires_at_str = format_rfc3339(expires_at);
        match mode {
            TestLockMode::Shared => {
                self.acquire_shared_lock(&key, token, &expires_at_str, &mut doc)
                    .await
            }
            TestLockMode::Exclusive => {
                self.acquire_exclusive_lock(&key, token, &expires_at_str, &mut doc)
                    .await
            }
        }
    }

    pub async fn admin_test_unlock(
        &self,
        event_id: i32,
        token: &str,
    ) -> Result<bool, StorageError> {
        let key = Self::kv_test_lock_key(event_id);
        let mut doc: TestLockDoc = match self.kv_get_json(&key).await {
            Ok(value) => value,
            Err(_) => return Ok(true),
        };

        let now = Utc::now().naive_utc();
        doc.shared_holders.remove(token);
        if let Some((holder, _)) = doc.exclusive_holder.clone() {
            if holder == token {
                doc.exclusive_holder = None;
            }
        }
        doc.shared_holders.retain(|_, expires_at| {
            parse_rfc3339(expires_at)
                .map(|ts| ts > now)
                .unwrap_or(false)
        });

        let is_last = doc.shared_holders.is_empty() && doc.exclusive_holder.is_none();
        if is_last {
            let _ = self.kv.delete(&key).await;
        } else {
            self.kv_put_json(&key, &doc).await?;
        }
        Ok(is_last)
    }

    pub async fn admin_test_unlock_all(&self) -> Result<(), StorageError> {
        let keys = self.kv_list_keys_with_prefix(Self::kv_test_lock_prefix()).await?;
        for key in keys {
            if key.ends_with(":test_lock") {
                let _ = self.kv.delete(&key).await;
            }
        }
        Ok(())
    }

    async fn acquire_shared_lock(
        &self,
        key: &str,
        token: &str,
        expires_at_str: &str,
        doc: &mut TestLockDoc,
    ) -> Result<(bool, bool), StorageError> {
        if doc.exclusive_holder.is_some() {
            return Ok((false, false));
        }
        let is_first = doc.shared_holders.is_empty();
        doc.shared_holders
            .insert(token.to_string(), expires_at_str.to_string());
        self.kv_put_json(key, doc).await?;
        Ok((true, is_first))
    }

    async fn acquire_exclusive_lock(
        &self,
        key: &str,
        token: &str,
        expires_at_str: &str,
        doc: &mut TestLockDoc,
    ) -> Result<(bool, bool), StorageError> {
        if doc.exclusive_holder.is_some() || !doc.shared_holders.is_empty() {
            return Ok((false, false));
        }
        doc.exclusive_holder = Some((token.to_string(), expires_at_str.to_string()));
        self.kv_put_json(key, doc).await?;
        Ok((true, true))
    }
}
