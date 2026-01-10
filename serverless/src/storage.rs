#![cfg(target_arch = "wasm32")]

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use rusty_golf_core::model::score::Statistic;
use rusty_golf_core::model::{RefreshSource, Scores, ScoresAndLastRefresh};
use rusty_golf_core::storage::{EventDetails, Storage, StorageError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use worker::{Bucket, Env, KvStore};

#[derive(Clone)]
pub struct ServerlessStorage {
    kv: KvStore,
    bucket: Bucket,
}

#[derive(Clone)]
pub struct EventListing {
    pub event_id: i32,
    pub event_name: String,
    pub score_view_step_factor: f32,
    pub refresh_from_espn: i64,
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
        Ok(Self { kv, bucket })
    }

    pub fn scores_key(event_id: i32) -> String {
        format!("events/{event_id}/scores.json")
    }

    pub fn espn_cache_key(event_id: i32) -> String {
        format!("cache/espn/{event_id}.json")
    }

    fn kv_event_details_key(event_id: i32) -> String {
        format!("event:{event_id}:details")
    }

    fn kv_golfers_key(event_id: i32) -> String {
        format!("event:{event_id}:golfers")
    }

    fn kv_player_factors_key(event_id: i32) -> String {
        format!("event:{event_id}:player_factors")
    }

    fn kv_last_refresh_key(event_id: i32) -> String {
        format!("event:{event_id}:last_refresh")
    }

    fn kv_seeded_at_key(event_id: i32, suffix: &str) -> String {
        format!("event:{event_id}:{suffix}:seeded_at")
    }

    async fn kv_get_json<T>(&self, key: &str) -> Result<T, StorageError>
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

    async fn kv_put_json<T>(&self, key: &str, value: &T) -> Result<(), StorageError>
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

    async fn r2_get_json<T>(&self, key: &str) -> Result<T, StorageError>
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

    async fn r2_put_json<T>(&self, key: &str, value: &T) -> Result<(), StorageError>
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

    async fn kv_list_keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
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

    pub async fn admin_seed_event(&self, request: AdminSeedRequest) -> Result<(), StorageError> {
        if request.event_id != request.event.event as i32 {
            return Err(StorageError::new(format!(
                "event_id mismatch: request {}, event {}",
                request.event_id, request.event.event
            )));
        }

        let data_to_fill = request
            .event
            .data_to_fill_if_event_and_year_missing
            .first()
            .ok_or_else(|| StorageError::new("missing data_to_fill_if_event_and_year_missing"))?;

        let details = EventDetailsDoc {
            event_name: request.event.name.clone(),
            score_view_step_factor: request.event.score_view_step_factor.as_f64().unwrap_or(0.0)
                as f32,
            refresh_from_espn: request.refresh_from_espn,
            end_date: request.event.end_date.clone(),
        };
        let details_key = Self::kv_event_details_key(request.event_id);
        self.kv_put_json(&details_key, &details).await?;

        let mut golfers_by_id = HashMap::new();
        for golfer in &data_to_fill.golfers {
            golfers_by_id.insert(golfer.espn_id, golfer.name.as_str());
        }

        let mut bettor_counts: HashMap<&str, usize> = HashMap::new();
        let mut golfers_out = Vec::new();
        let mut eup_id = 1_i64;
        for entry in &data_to_fill.event_user_player {
            let count = bettor_counts.entry(entry.bettor.as_str()).or_insert(0);
            *count += 1;

            let golfer_name = golfers_by_id
                .get(&entry.golfer_espn_id)
                .ok_or_else(|| {
                    StorageError::new(format!(
                        "missing golfer_espn_id {} for event {}",
                        entry.golfer_espn_id, request.event.event
                    ))
                })?;

            golfers_out.push(GolferAssignment {
                eup_id,
                espn_id: entry.golfer_espn_id,
                golfer_name: (*golfer_name).to_string(),
                bettor_name: entry.bettor.clone(),
                group: *count as i64,
                score_view_step_factor: entry
                    .score_view_step_factor
                    .as_ref()
                    .and_then(|value| value.as_f64().map(|num| num as f32)),
            });
            eup_id += 1;
        }

        let golfers_key = Self::kv_golfers_key(request.event_id);
        self.kv_put_json(&golfers_key, &golfers_out).await?;

        let player_factors: Vec<PlayerFactorEntry> = data_to_fill
            .event_user_player
            .iter()
            .filter_map(|entry| {
                entry.score_view_step_factor.as_ref().and_then(|factor| {
                    factor.as_f64().map(|num| PlayerFactorEntry {
                        golfer_espn_id: entry.golfer_espn_id,
                        bettor_name: entry.bettor.clone(),
                        step_factor: num as f32,
                    })
                })
            })
            .collect();
        let factors_key = Self::kv_player_factors_key(request.event_id);
        self.kv_put_json(&factors_key, &player_factors).await?;

        if let Some(tokens) = request.auth_tokens.as_ref() {
            let auth_doc = AuthTokensDoc {
                tokens: tokens.clone(),
            };
            let auth_key = format!("event:{}:auth_tokens", request.event_id);
            self.kv_put_json(&auth_key, &auth_doc).await?;
        }

        let last_refresh_ts = if let Some(ts) = request.last_refresh.as_ref() {
            parse_rfc3339(ts).map_err(|e| StorageError::new(e.to_string()))?
        } else {
            Utc::now().naive_utc()
        };

        let scores_payload = ScoresAndLastRefresh {
            score_struct: request.score_struct,
            last_refresh: last_refresh_ts,
            last_refresh_source: RefreshSource::Espn,
        };
        let scores_key = Self::scores_key(request.event_id);
        self.r2_put_json(&scores_key, &scores_payload).await?;

        let cache_key = Self::espn_cache_key(request.event_id);
        self.r2_put_json(&cache_key, &request.espn_cache).await?;

        let last_refresh_doc = LastRefreshDoc {
            ts: format_rfc3339(last_refresh_ts),
            source: RefreshSource::Espn,
        };
        let last_refresh_key = Self::kv_last_refresh_key(request.event_id);
        self.kv_put_json(&last_refresh_key, &last_refresh_doc).await?;

        let seeded_at = SeededAtDoc {
            seeded_at: format_rfc3339(Utc::now().naive_utc()),
        };
        let seeded_keys = [
            Self::kv_seeded_at_key(request.event_id, "details"),
            Self::kv_seeded_at_key(request.event_id, "golfers"),
            Self::kv_seeded_at_key(request.event_id, "player_factors"),
            Self::kv_seeded_at_key(request.event_id, "last_refresh"),
        ];
        for key in seeded_keys {
            self.kv_put_json(&key, &seeded_at).await?;
        }

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
        let mut details: EventDetailsDoc = self.kv_get_json(&details_key).await?;
        details.end_date = end_date;
        self.kv_put_json(&details_key, &details).await?;

        let seeded_at = SeededAtDoc {
            seeded_at: format_rfc3339(Utc::now().naive_utc()),
        };
        let seeded_key = Self::kv_seeded_at_key(event_id, "details");
        let _ = self.kv_put_json(&seeded_key, &seeded_at).await;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct EventDetailsDoc {
    event_name: String,
    score_view_step_factor: f32,
    refresh_from_espn: i64,
    end_date: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct GolferAssignment {
    eup_id: i64,
    espn_id: i64,
    golfer_name: String,
    bettor_name: String,
    group: i64,
    score_view_step_factor: Option<f32>,
}

#[derive(Serialize, Deserialize)]
struct PlayerFactorEntry {
    golfer_espn_id: i64,
    bettor_name: String,
    step_factor: f32,
}

#[derive(Serialize, Deserialize)]
struct AuthTokensDoc {
    tokens: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct LastRefreshDoc {
    ts: String,
    source: RefreshSource,
}

#[derive(Serialize, Deserialize)]
struct SeededAtDoc {
    seeded_at: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminSeedRequest {
    pub event_id: i32,
    pub refresh_from_espn: i64,
    pub event: AdminEupEvent,
    pub score_struct: Vec<Scores>,
    pub espn_cache: serde_json::Value,
    pub auth_tokens: Option<Vec<String>>,
    pub last_refresh: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminEupEvent {
    pub event: i64,
    pub name: String,
    pub score_view_step_factor: serde_json::Value,
    pub end_date: Option<String>,
    pub data_to_fill_if_event_and_year_missing: Vec<AdminEupDataFill>,
}

#[derive(Debug, Deserialize)]
pub struct AdminEupDataFill {
    pub golfers: Vec<AdminEupGolfer>,
    pub event_user_player: Vec<AdminEupEventUserPlayer>,
}

#[derive(Debug, Deserialize)]
pub struct AdminEupGolfer {
    pub espn_id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminEupEventUserPlayer {
    pub bettor: String,
    pub golfer_espn_id: i64,
    pub score_view_step_factor: Option<serde_json::Value>,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl Storage for ServerlessStorage {
    async fn get_event_details(&self, event_id: i32) -> Result<EventDetails, StorageError> {
        let key = Self::kv_event_details_key(event_id);
        let doc: EventDetailsDoc = self.kv_get_json(&key).await?;
        Ok(EventDetails {
            event_name: doc.event_name,
            score_view_step_factor: doc.score_view_step_factor,
            refresh_from_espn: doc.refresh_from_espn,
            end_date: doc.end_date,
        })
    }

    async fn get_golfers_for_event(&self, event_id: i32) -> Result<Vec<Scores>, StorageError> {
        let key = Self::kv_golfers_key(event_id);
        let assignments: Vec<GolferAssignment> = self.kv_get_json(&key).await?;
        Ok(assignments
            .into_iter()
            .map(|assignment| Scores {
                eup_id: assignment.eup_id,
                espn_id: assignment.espn_id,
                golfer_name: assignment.golfer_name,
                bettor_name: assignment.bettor_name,
                detailed_statistics: Statistic {
                    eup_id: assignment.eup_id,
                    rounds: Vec::new(),
                    round_scores: Vec::new(),
                    tee_times: Vec::new(),
                    holes_completed_by_round: Vec::new(),
                    line_scores: Vec::new(),
                    total_score: 0,
                },
                group: assignment.group,
                score_view_step_factor: assignment.score_view_step_factor,
            })
            .collect())
    }

    async fn get_player_step_factors(
        &self,
        event_id: i32,
    ) -> Result<HashMap<(i64, String), f32>, StorageError> {
        let key = Self::kv_player_factors_key(event_id);
        let entries: Vec<PlayerFactorEntry> = self.kv_get_json(&key).await?;
        Ok(entries
            .into_iter()
            .map(|entry| ((entry.golfer_espn_id, entry.bettor_name), entry.step_factor))
            .collect())
    }

    async fn get_scores(
        &self,
        event_id: i32,
        source: RefreshSource,
    ) -> Result<ScoresAndLastRefresh, StorageError> {
        let key = Self::scores_key(event_id);
        let mut scores: ScoresAndLastRefresh = self.r2_get_json(&key).await?;
        scores.last_refresh_source = source;
        Ok(scores)
    }

    async fn store_scores(&self, event_id: i32, scores: &[Scores]) -> Result<(), StorageError> {
        let now = Utc::now().naive_utc();
        let payload = ScoresAndLastRefresh {
            score_struct: scores.to_vec(),
            last_refresh: now,
            last_refresh_source: RefreshSource::Espn,
        };
        let key = Self::scores_key(event_id);
        self.r2_put_json(&key, &payload).await?;

        let last_refresh = LastRefreshDoc {
            ts: format_rfc3339(now),
            source: RefreshSource::Espn,
        };
        let kv_key = Self::kv_last_refresh_key(event_id);
        self.kv_put_json(&kv_key, &last_refresh).await?;

        let seeded_at = SeededAtDoc {
            seeded_at: format_rfc3339(now),
        };
        let seeded_key = Self::kv_seeded_at_key(event_id, "last_refresh");
        self.kv_put_json(&seeded_key, &seeded_at).await?;
        Ok(())
    }

    async fn event_and_scores_already_in_db(
        &self,
        event_id: i32,
        max_age_seconds: i64,
    ) -> Result<bool, StorageError> {
        if max_age_seconds <= 0 {
            return Ok(false);
        }
        let details_key = Self::kv_event_details_key(event_id);
        if self
            .kv
            .get(&details_key)
            .text()
            .await
            .ok()
            .flatten()
            .is_none()
        {
            return Ok(false);
        }

        let last_refresh_key = Self::kv_last_refresh_key(event_id);
        let last_refresh: LastRefreshDoc = match self.kv_get_json(&last_refresh_key).await {
            Ok(val) => val,
            Err(_) => return Ok(false),
        };

        let last_refresh_ts =
            parse_rfc3339(&last_refresh.ts).map_err(|e| StorageError::new(e.to_string()))?;
        let now = Utc::now().naive_utc();
        let diff = now.signed_duration_since(last_refresh_ts);
        Ok(diff.num_seconds() <= max_age_seconds)
    }
}

fn parse_rfc3339(value: &str) -> Result<NaiveDateTime, chrono::ParseError> {
    Ok(DateTime::parse_from_rfc3339(value)?.naive_utc())
}

fn format_rfc3339(value: NaiveDateTime) -> String {
    DateTime::<Utc>::from_naive_utc_and_offset(value, Utc).to_rfc3339()
}

fn parse_event_id(key: &str, suffix: &str) -> Option<i32> {
    let prefix = "event:";
    if !key.starts_with(prefix) || !key.ends_with(suffix) {
        return None;
    }
    let start = prefix.len();
    let end = key.len().saturating_sub(suffix.len());
    key.get(start..end)?.parse().ok()
}
