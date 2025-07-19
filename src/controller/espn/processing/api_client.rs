use futures::future::join_all;
use crate::controller::espn::client::get_json_from_espn;
use crate::model::{PlayerJsonResponse, Scores};

pub async fn get_espn_data_parallel(
    scores: &[Scores],
    year: i32,
    event_id: i32,
) -> Result<PlayerJsonResponse, Box<dyn std::error::Error>> {
    if cfg!(debug_assertions) {
        return get_json_from_espn(scores, year, event_id).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
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

        let player_group_clone = player_group.clone();
        let future = tokio::task::spawn(async move {
            match get_json_from_espn(&player_group_clone, year, event_id).await {
                Ok(response) => Some(response),
                Err(err) => {
                    eprintln!("Failed to get ESPN data: {err}");
                    None
                }
            }
        });

        futures.push(future);
    }

    let results = join_all(futures).await;

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