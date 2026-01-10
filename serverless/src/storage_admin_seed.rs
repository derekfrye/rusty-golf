#![cfg(target_arch = "wasm32")]

use chrono::Utc;
use rusty_golf_core::model::{RefreshSource, ScoresAndLastRefresh};
use rusty_golf_core::storage::StorageError;

use crate::storage::ServerlessStorage;
use crate::storage_admin_seed_helpers::{
    build_golfers_out, build_player_factors, resolve_last_refresh_ts, validate_seed_request,
};
use crate::storage_helpers::format_rfc3339;
use crate::storage_types::{
    AdminSeedRequest, AuthTokensDoc, LastRefreshDoc, SeededAtDoc,
};

impl ServerlessStorage {
    pub async fn admin_seed_event(&self, request: AdminSeedRequest) -> Result<(), StorageError> {
        let data_to_fill = validate_seed_request(&request)?;
        self.store_event_details(&request).await?;

        let golfers_out = build_golfers_out(request.event_id, data_to_fill)?;
        let golfers_key = Self::kv_golfers_key(request.event_id);
        self.kv_put_json(&golfers_key, &golfers_out).await?;

        let player_factors = build_player_factors(data_to_fill);
        let factors_key = Self::kv_player_factors_key(request.event_id);
        self.kv_put_json(&factors_key, &player_factors).await?;

        self.store_auth_tokens(request.event_id, request.auth_tokens.as_ref())
            .await?;

        let last_refresh_ts = resolve_last_refresh_ts(&request)?;
        self.store_scores_and_cache(request.event_id, &request, last_refresh_ts)
            .await?;
        self.store_last_refresh_doc(request.event_id, last_refresh_ts)
            .await?;
        self.store_seeded_at_docs(request.event_id).await?;

        Ok(())
    }

    pub async fn admin_cleanup_event(
        &self,
        event_id: i32,
        include_auth_tokens: bool,
    ) -> Result<(), StorageError> {
        let kv_keys = [
            Self::kv_event_details_key(event_id),
            Self::kv_golfers_key(event_id),
            Self::kv_player_factors_key(event_id),
            Self::kv_last_refresh_key(event_id),
            Self::kv_seeded_at_key(event_id, "details"),
            Self::kv_seeded_at_key(event_id, "golfers"),
            Self::kv_seeded_at_key(event_id, "player_factors"),
            Self::kv_seeded_at_key(event_id, "last_refresh"),
        ];
        for key in kv_keys {
            let _ = self.kv.delete(&key).await;
        }

        if include_auth_tokens {
            let auth_key = format!("event:{event_id}:auth_tokens");
            let _ = self.kv.delete(&auth_key).await;
        }

        let scores_key = Self::scores_key(event_id);
        let cache_key = Self::espn_cache_key(event_id);
        let _ = self.bucket.delete(scores_key).await;
        let _ = self.bucket.delete(cache_key).await;
        Ok(())
    }

    pub async fn admin_update_event_end_date(
        &self,
        event_id: i32,
        end_date: Option<String>,
    ) -> Result<(), StorageError> {
        let details_key = Self::kv_event_details_key(event_id);
        let mut details: crate::storage_types::EventDetailsDoc =
            self.kv_get_json(details_key.as_str()).await?;
        details.end_date = end_date;
        self.kv_put_json(&details_key, &details).await?;

        let seeded_at = SeededAtDoc {
            seeded_at: format_rfc3339(Utc::now().naive_utc()),
        };
        let seeded_key = Self::kv_seeded_at_key(event_id, "details");
        let _ = self.kv_put_json(&seeded_key, &seeded_at).await;
        Ok(())
    }

    async fn store_event_details(&self, request: &AdminSeedRequest) -> Result<(), StorageError> {
        let details = crate::storage_types::EventDetailsDoc {
            event_name: request.event.name.clone(),
            score_view_step_factor: request.event.score_view_step_factor.as_f64().unwrap_or(0.0)
                as f32,
            refresh_from_espn: request.refresh_from_espn,
            end_date: request.event.end_date.clone(),
        };
        let details_key = Self::kv_event_details_key(request.event_id);
        self.kv_put_json(&details_key, &details).await
    }

    async fn store_auth_tokens(
        &self,
        event_id: i32,
        tokens: Option<&Vec<String>>,
    ) -> Result<(), StorageError> {
        let Some(tokens) = tokens else {
            return Ok(());
        };
        let auth_doc = AuthTokensDoc {
            tokens: tokens.clone(),
        };
        let auth_key = format!("event:{event_id}:auth_tokens");
        self.kv_put_json(&auth_key, &auth_doc).await
    }

    async fn store_scores_and_cache(
        &self,
        event_id: i32,
        request: &AdminSeedRequest,
        last_refresh_ts: chrono::NaiveDateTime,
    ) -> Result<(), StorageError> {
        let scores_payload = ScoresAndLastRefresh {
            score_struct: request.score_struct.clone(),
            last_refresh: last_refresh_ts,
            last_refresh_source: RefreshSource::Espn,
        };
        let scores_key = Self::scores_key(event_id);
        self.r2_put_json(&scores_key, &scores_payload).await?;

        let cache_key = Self::espn_cache_key(event_id);
        self.r2_put_json(&cache_key, &request.espn_cache).await?;
        Ok(())
    }

    async fn store_last_refresh_doc(
        &self,
        event_id: i32,
        last_refresh_ts: chrono::NaiveDateTime,
    ) -> Result<(), StorageError> {
        let last_refresh_doc = LastRefreshDoc {
            ts: format_rfc3339(last_refresh_ts),
            source: RefreshSource::Espn,
        };
        let last_refresh_key = Self::kv_last_refresh_key(event_id);
        self.kv_put_json(&last_refresh_key, &last_refresh_doc).await
    }

    async fn store_seeded_at_docs(&self, event_id: i32) -> Result<(), StorageError> {
        let seeded_at = SeededAtDoc {
            seeded_at: format_rfc3339(Utc::now().naive_utc()),
        };
        let seeded_keys = [
            Self::kv_seeded_at_key(event_id, "details"),
            Self::kv_seeded_at_key(event_id, "golfers"),
            Self::kv_seeded_at_key(event_id, "player_factors"),
            Self::kv_seeded_at_key(event_id, "last_refresh"),
        ];
        for key in seeded_keys {
            self.kv_put_json(&key, &seeded_at).await?;
        }
        Ok(())
    }
}
