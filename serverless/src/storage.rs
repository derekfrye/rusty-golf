#![cfg(target_arch = "wasm32")]

use rusty_golf_core::storage::StorageError;
use rusty_golf_core::timing::TimingSink;
use std::rc::Rc;
use worker::{Bucket, Env, KvStore};

mod storage_admin_lock;
mod storage_admin_seed;
mod storage_admin_seed_helpers;
pub(crate) mod storage_cache;
mod storage_completion;
mod storage_helpers;
mod storage_impl;
mod storage_kv;
mod storage_kv_keys;
mod storage_kv_listing;
mod storage_r2;
mod storage_types;

pub use storage_helpers::{format_rfc3339, parse_event_id, parse_rfc3339};
pub use storage_types::{
    AdminEupDataFill, AdminEupEvent, AdminEupEventUserPlayer, AdminEupGolfer, AdminSeedRequest,
    AuthTokensDoc, EventDetailsDoc, EventListing, GolferAssignment, LastRefreshDoc,
    PlayerFactorEntry, SeededAtDoc, TestLockDoc, TestLockMode,
};

#[derive(Clone)]
pub struct ServerlessStorage {
    pub(crate) kv: KvStore,
    pub(crate) bucket: Bucket,
    pub(crate) timing: Option<Rc<dyn TimingSink>>,
}

impl ServerlessStorage {
    pub const KV_BINDING: &str = "djf_rusty_golf_kv";
    pub const R2_BINDING: &str = "SCORES_R2";

    pub fn from_env(env: &Env, kv_binding: &str, r2_binding: &str) -> Result<Self, StorageError> {
        let kv = env
            .kv(kv_binding)
            .map_err(|e| StorageError::new(e.to_string()))?;
        let bucket = env
            .bucket(r2_binding)
            .map_err(|e| StorageError::new(e.to_string()))?;
        Ok(Self {
            kv,
            bucket,
            timing: None,
        })
    }

    #[must_use]
    pub fn with_timing(mut self, timing: Option<Rc<dyn TimingSink>>) -> Self {
        self.timing = timing;
        self
    }

    pub fn timing(&self) -> Option<&dyn TimingSink> {
        self.timing.as_deref()
    }
}
