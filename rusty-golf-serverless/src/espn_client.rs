#![cfg(target_arch = "wasm32")]

use futures::{StreamExt, TryStreamExt, stream};
use rusty_golf_core::error::CoreError;
use rusty_golf_core::espn::EspnApiClient;
use rusty_golf_core::model::{PlayerJsonResponse, Scores};
use std::collections::HashMap;
use worker::{Fetch, Url};

pub struct ServerlessEspnClient;

const ESPN_FETCH_FANOUT: usize = 6;

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
        let fetches = stream::iter(scores.iter().map(|score| {
            let espn_id = score.espn_id;
            let eup_id = score.eup_id;
            async move {
                let url = format!(
                    "https://site.web.api.espn.com/apis/site/v2/sports/golf/pga/leaderboard/{}/playersummary?season={}&player={}",
                    event_id, year, espn_id
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
                    Ok::<Option<(i64, HashMap<String, serde_json::Value>)>, CoreError>(Some((
                        eup_id, json,
                    )))
                } else {
                    Ok::<Option<(i64, HashMap<String, serde_json::Value>)>, CoreError>(None)
                }
            }
        }))
        .buffer_unordered(ESPN_FETCH_FANOUT)
        .try_collect::<Vec<_>>()
        .await?;

        let mut player_response = PlayerJsonResponse {
            data: Vec::new(),
            eup_ids: Vec::new(),
        };

        for maybe_response in fetches {
            if let Some((eup_id, json)) = maybe_response {
                player_response.data.push(json);
                player_response.eup_ids.push(eup_id);
            }
        }

        Ok(player_response)
    }
}
