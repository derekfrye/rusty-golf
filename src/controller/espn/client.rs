use crate::model::{PlayerJsonResponse, Scores};
use reqwest::Client;
use std::collections::HashMap;


/// # Errors
///
/// Will return `Err` if the espn api call fails
pub async fn get_json_from_espn(
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
