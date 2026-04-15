mod fetch;
pub mod processing;

use crate::error::CoreError;
use crate::model::{PlayerJsonResponse, Scores};
use async_trait::async_trait;

pub use fetch::{
    FetchScoresRequest, fetch_scores_from_espn, fetch_scores_from_espn_with_timing,
    go_get_espn_data, go_get_espn_data_with_timing,
};

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait EspnApiClient: Send + Sync {
    async fn get_json_from_espn(
        &self,
        scores: &[Scores],
        year: i32,
        event_id: i32,
    ) -> Result<PlayerJsonResponse, CoreError>;

    async fn fallback_scores(&self, _event_id: i32) -> Result<Option<Vec<Scores>>, CoreError> {
        Ok(None)
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait EspnApiClient {
    async fn get_json_from_espn(
        &self,
        scores: &[Scores],
        year: i32,
        event_id: i32,
    ) -> Result<PlayerJsonResponse, CoreError>;

    async fn fallback_scores(&self, _event_id: i32) -> Result<Option<Vec<Scores>>, CoreError> {
        Ok(None)
    }
}
