#![cfg(target_arch = "wasm32")]

use rusty_golf_core::error::CoreError;
use rusty_golf_core::espn::EspnApiClient;
use rusty_golf_core::model::{PlayerJsonResponse, Scores};
use std::collections::HashMap;
use worker::{Fetch, Url};

pub struct ServerlessEspnClient;

impl ServerlessEspnClient {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl EspnApiClient for ServerlessEspnClient {
    async fn get_json_from_espn(
        &self,
        scores: &[Scores],
        year: i32,
        event_id: i32,
    ) -> Result<PlayerJsonResponse, CoreError> {
        let mut player_response = PlayerJsonResponse {
            data: Vec::new(),
            eup_ids: Vec::new(),
        };

        for score in scores {
            let url = format!(
                "https://site.web.api.espn.com/apis/site/v2/sports/golf/pga/leaderboard/{}/playersummary?season={}&player={}",
                event_id, year, score.espn_id
            );
            let url = Url::parse(&url).map_err(|e| CoreError::Network(e.to_string()))?;
            let mut resp = Fetch::Url(url)
                .send()
                .await
                .map_err(|e| CoreError::Network(e.to_string()))?;
            let json: HashMap<String, serde_json::Value> = resp
                .json()
                .await
                .map_err(|e| CoreError::Network(e.to_string()))?;

            if json.contains_key("rounds") {
                player_response.data.push(json);
                player_response.eup_ids.push(score.eup_id);
            }
        }

        Ok(player_response)
    }
}
