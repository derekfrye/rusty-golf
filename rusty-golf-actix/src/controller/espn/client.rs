use crate::model::{PlayerJsonResponse, Scores};
use reqwest::Client;
use rusty_golf_core::error::CoreError;
use rusty_golf_core::espn::EspnApiClient;
use std::collections::HashMap;

pub struct ActixEspnClient;

impl ActixEspnClient {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for ActixEspnClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl EspnApiClient for ActixEspnClient {
    async fn get_json_from_espn(
        &self,
        scores: &[Scores],
        year: i32,
        event_id: i32,
    ) -> Result<PlayerJsonResponse, CoreError> {
        if cfg!(debug_assertions) {
            return get_json_from_espn(scores, year, event_id)
                .await
                .map_err(|e| CoreError::Network(e.to_string()));
        }

        let num_scores = scores.len();
        let group_size = num_scores.div_ceil(4);
        let mut futures = Vec::with_capacity(4);

        for task_index in 0..4 {
            let player_group = scores
                .iter()
                .skip(task_index * group_size)
                .take(group_size)
                .cloned()
                .collect::<Vec<_>>();

            if player_group.is_empty() {
                continue;
            }

            let future = tokio::task::spawn(async move {
                match get_json_from_espn(&player_group, year, event_id).await {
                    Ok(response) => Some(response),
                    Err(err) => {
                        eprintln!("Failed to get ESPN data: {err}");
                        None
                    }
                }
            });

            futures.push(future);
        }

        let results = futures::future::join_all(futures).await;
        let mut combined_response = PlayerJsonResponse {
            data: Vec::new(),
            eup_ids: Vec::new(),
        };

        for response in results.into_iter().flatten().flatten() {
            combined_response.data.extend(response.data);
            combined_response.eup_ids.extend(response.eup_ids);
        }

        Ok(combined_response)
    }

    async fn fallback_scores(&self, event_id: i32) -> Result<Option<Vec<Scores>>, CoreError> {
        let text = std::fs::read_to_string("tests/test3_espn_json_responses.json")?;
        let val = serde_json::from_str::<serde_json::Value>(&text)?;
        let score_struct = val
            .get("score_struct")
            .ok_or_else(|| CoreError::Other("offline fixture missing score_struct".into()))?;
        let scores_vec = serde_json::from_value::<Vec<Scores>>(score_struct.clone())?;
        eprintln!("ESPN fetch failed: falling back to offline fixtures for event {event_id}.");
        Ok(Some(scores_vec))
    }
}

async fn get_json_from_espn(
    scores: &[Scores],
    year: i32,
    event_id: i32,
) -> Result<PlayerJsonResponse, reqwest::Error> {
    let client = Client::new();
    let mut player_response = PlayerJsonResponse {
        data: Vec::new(),
        eup_ids: Vec::new(),
    };

    for score in scores {
        let url = format!(
            "https://site.web.api.espn.com/apis/site/v2/sports/golf/pga/leaderboard/{}/playersummary?season={}&player={}",
            event_id, year, score.espn_id
        );

        let resp = client.get(&url).send().await?;
        let json: HashMap<String, serde_json::Value> = resp.json().await?;

        if json.contains_key("rounds") {
            player_response.data.push(json);
            player_response.eup_ids.push(score.eup_id);
        }
    }

    Ok(player_response)
}
