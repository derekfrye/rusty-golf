use chrono::{NaiveDateTime, Utc};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use rusty_golf_core::storage::{EventDetails, Storage, StorageError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use aws_sign_v4::AwsSign;
use reqwest::Url;
use sha256::digest as sha256_digest;

use crate::model::{RefreshSource, Scores, ScoresAndLastRefresh, Statistic};

#[derive(Clone, Debug)]
pub struct R2StorageConfig {
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub service: String,
}

#[derive(Clone)]
pub struct R2Storage {
    client: reqwest::Client,
    config: R2StorageConfig,
    signer: Arc<dyn S3Signer>,
}

impl R2Storage {
    #[must_use]
    pub fn new(config: R2StorageConfig, signer: Arc<dyn S3Signer>) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
            signer,
        }
    }

    /// Build config from `.env` and process environment variables.
    ///
    /// Required:
    /// - `R2_ENDPOINT`, `R2_BUCKET`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`
    ///
    /// Optional:
    /// - `R2_REGION` (defaults to `auto`)
    /// - `R2_SERVICE` (defaults to `s3`)
    ///
    /// # Errors
    /// Returns an error if required environment variables are missing.
    pub fn config_from_env() -> Result<R2StorageConfig, StorageError> {
        dotenvy::dotenv().ok();

        let endpoint = std::env::var("R2_ENDPOINT")
            .map_err(|_| StorageError::new("missing R2_ENDPOINT"))?;
        let bucket = std::env::var("R2_BUCKET")
            .map_err(|_| StorageError::new("missing R2_BUCKET"))?;
        let access_key_id = std::env::var("R2_ACCESS_KEY_ID")
            .or_else(|_| std::env::var("AWS_ACCESS_KEY_ID"))
            .map_err(|_| StorageError::new("missing R2_ACCESS_KEY_ID"))?;
        let secret_access_key = std::env::var("R2_SECRET_ACCESS_KEY")
            .or_else(|_| std::env::var("AWS_SECRET_ACCESS_KEY"))
            .map_err(|_| StorageError::new("missing R2_SECRET_ACCESS_KEY"))?;
        let region = std::env::var("R2_REGION").unwrap_or_else(|_| "auto".to_string());
        let service = std::env::var("R2_SERVICE").unwrap_or_else(|_| "s3".to_string());

        Ok(R2StorageConfig {
            endpoint,
            bucket,
            region,
            access_key_id,
            secret_access_key,
            service,
        })
    }

    #[must_use]
    pub fn signer_from_config(config: &R2StorageConfig) -> SigV4Signer {
        SigV4Signer::new(
            config.access_key_id.clone(),
            config.secret_access_key.clone(),
            config.region.clone(),
            config.service.clone(),
        )
    }

    fn object_url(&self, key: &str) -> String {
        let base = self.config.endpoint.trim_end_matches('/');
        let bucket = self.config.bucket.trim_matches('/');
        format!("{base}/{bucket}/{key}")
    }

    async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let url = self.object_url(key);
        let headers = self.signer.sign("GET", &url, HeaderMap::new(), None)?;
        let resp = self
            .client
            .get(url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| StorageError::new(e.to_string()))?;

        if resp.status().as_u16() == 404 {
            return Ok(None);
        }

        if !resp.status().is_success() {
            return Err(StorageError::new(format!(
                "R2 GET failed with status {}",
                resp.status()
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| StorageError::new(e.to_string()))?;
        Ok(Some(bytes.to_vec()))
    }

    async fn put_object(&self, key: &str, body: Vec<u8>) -> Result<(), StorageError> {
        let url = self.object_url(key);
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let headers = self
            .signer
            .sign("PUT", &url, headers, Some(&body))?;

        let resp = self
            .client
            .put(url)
            .headers(headers)
            .body(body)
            .send()
            .await
            .map_err(|e| StorageError::new(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(StorageError::new(format!(
                "R2 PUT failed with status {}",
                resp.status()
            )));
        }

        Ok(())
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, StorageError> {
        let Some(bytes) = self.get_object(key).await? else {
            return Ok(None);
        };
        let parsed = serde_json::from_slice::<T>(&bytes)
            .map_err(|e| StorageError::new(e.to_string()))?;
        Ok(Some(parsed))
    }

    async fn put_json<T: Serialize>(&self, key: &str, value: &T) -> Result<(), StorageError> {
        let body =
            serde_json::to_vec(value).map_err(|e| StorageError::new(e.to_string()))?;
        self.put_object(key, body).await
    }

    fn scores_key(event_id: i32) -> String {
        format!("events/{event_id}/scores.json")
    }

    fn golfers_key(event_id: i32) -> String {
        format!("events/{event_id}/golfers.json")
    }

    fn event_key(event_id: i32) -> String {
        format!("events/{event_id}/event.json")
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct R2EventDetails {
    event_name: String,
    score_view_step_factor: f32,
    refresh_from_espn: i64,
}

#[derive(Debug)]
pub struct MissingSigner;

impl S3Signer for MissingSigner {
    fn sign(
        &self,
        _method: &str,
        _url: &str,
        _headers: HeaderMap,
        _body: Option<&[u8]>,
    ) -> Result<HeaderMap, StorageError> {
        Err(StorageError::new(
            "S3 signer not configured for R2Storage",
        ))
    }
}

#[derive(Clone)]
pub struct SigV4Signer {
    access_key_id: String,
    secret_access_key: String,
    region: String,
    service: String,
}

impl SigV4Signer {
    #[must_use]
    pub fn new(
        access_key_id: String,
        secret_access_key: String,
        region: String,
        service: String,
    ) -> Self {
        Self {
            access_key_id,
            secret_access_key,
            region,
            service,
        }
    }
}

pub trait S3Signer: Send + Sync {
    /// Sign a request and return the headers to attach.
    ///
    /// # Errors
    /// Returns an error if the request cannot be signed.
    fn sign(
        &self,
        method: &str,
        url: &str,
        headers: HeaderMap,
        body: Option<&[u8]>,
    ) -> Result<HeaderMap, StorageError>;
}

impl S3Signer for SigV4Signer {
    fn sign(
        &self,
        method: &str,
        url: &str,
        mut headers: HeaderMap,
        body: Option<&[u8]>,
    ) -> Result<HeaderMap, StorageError> {
        let body = body.unwrap_or(&[]);
        let url = Url::parse(url)
            .map_err(|e| StorageError::new(format!("invalid url: {e}")))?;

        let host = url
            .host_str()
            .ok_or_else(|| StorageError::new("missing host in url"))?;
        let now = Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let payload_hash = sha256_digest(body);

        headers.insert(
            HeaderName::from_static("host"),
            HeaderValue::from_str(host)
                .map_err(|e| StorageError::new(format!("invalid host header: {e}")))?,
        );
        headers.insert(
            HeaderName::from_static("x-amz-date"),
            HeaderValue::from_str(&amz_date)
                .map_err(|e| StorageError::new(format!("invalid x-amz-date: {e}")))?,
        );
        headers.insert(
            HeaderName::from_static("x-amz-content-sha256"),
            HeaderValue::from_str(&payload_hash)
                .map_err(|e| StorageError::new(format!("invalid x-amz-content-sha256: {e}")))?,
        );

        let signer = AwsSign::new(
            method,
            url.as_str(),
            &now,
            &headers,
            &self.region,
            &self.access_key_id,
            &self.secret_access_key,
            &self.service,
            body,
        );
        let auth_header = signer.sign();
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&auth_header)
                .map_err(|e| StorageError::new(format!("invalid authorization: {e}")))?,
        );

        Ok(headers)
    }
}

#[async_trait::async_trait]
impl Storage for R2Storage {
    async fn get_event_details(&self, event_id: i32) -> Result<EventDetails, StorageError> {
        let key = Self::event_key(event_id);
        let details = self
            .get_json::<R2EventDetails>(&key)
            .await?
            .ok_or_else(|| StorageError::new("event details not found"))?;

        Ok(EventDetails {
            event_name: details.event_name,
            score_view_step_factor: details.score_view_step_factor,
            refresh_from_espn: details.refresh_from_espn,
        })
    }

    async fn get_golfers_for_event(&self, event_id: i32) -> Result<Vec<Scores>, StorageError> {
        let golfers_key = Self::golfers_key(event_id);
        if let Some(golfers) = self.get_json::<Vec<Scores>>(&golfers_key).await? {
            return Ok(golfers);
        }

        let scores_key = Self::scores_key(event_id);
        if let Some(scores) = self.get_json::<ScoresAndLastRefresh>(&scores_key).await? {
            let golfers = scores
                .score_struct
                .iter()
                .map(|score| Scores {
                    detailed_statistics: Statistic {
                        eup_id: score.eup_id,
                        rounds: Vec::new(),
                        round_scores: Vec::new(),
                        tee_times: Vec::new(),
                        holes_completed_by_round: Vec::new(),
                        line_scores: Vec::new(),
                        total_score: 0,
                    },
                    ..score.clone()
                })
                .collect();
            return Ok(golfers);
        }

        Err(StorageError::new("golfers not found"))
    }

    async fn get_player_step_factors(
        &self,
        event_id: i32,
    ) -> Result<HashMap<(i64, String), f32>, StorageError> {
        let scores_key = Self::scores_key(event_id);
        let scores = self
            .get_json::<ScoresAndLastRefresh>(&scores_key)
            .await?
            .ok_or_else(|| StorageError::new("scores not found"))?;

        let step_factors = scores
            .score_struct
            .iter()
            .filter_map(|score| {
                let step = score.score_view_step_factor?;
                Some(((score.espn_id, score.bettor_name.clone()), step))
            })
            .collect();

        Ok(step_factors)
    }

    async fn get_scores(
        &self,
        event_id: i32,
        source: RefreshSource,
    ) -> Result<ScoresAndLastRefresh, StorageError> {
        let key = Self::scores_key(event_id);
        let mut scores = self
            .get_json::<ScoresAndLastRefresh>(&key)
            .await?
            .ok_or_else(|| StorageError::new("scores not found"))?;
        scores.last_refresh_source = source;
        Ok(scores)
    }

    async fn store_scores(
        &self,
        event_id: i32,
        scores: &[Scores],
    ) -> Result<(), StorageError> {
        let payload = ScoresAndLastRefresh {
            score_struct: scores.to_vec(),
            last_refresh: chrono::Utc::now().naive_utc(),
            last_refresh_source: RefreshSource::Espn,
        };
        self.put_json(&Self::scores_key(event_id), &payload).await
    }

    async fn event_and_scores_already_in_db(
        &self,
        event_id: i32,
        max_age_seconds: i64,
    ) -> Result<bool, StorageError> {
        let key = Self::scores_key(event_id);
        let Some(scores) = self.get_json::<ScoresAndLastRefresh>(&key).await? else {
            return Ok(false);
        };

        let now = chrono::Utc::now().naive_utc();
        let last_refresh: NaiveDateTime = scores.last_refresh;
        // Match SQL behavior: compare cache age in days.
        let diff = now.signed_duration_since(last_refresh);
        Ok(diff.num_days() >= max_age_seconds)
    }
}
