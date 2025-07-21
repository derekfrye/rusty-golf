pub mod api_client;
pub mod data_processor;
pub mod score_calculator;
pub mod time_processor;

use crate::model::Scores;
use api_client::get_espn_data_parallel;
use data_processor::{merge_statistics_with_scores, process_json_to_statistics};

pub use score_calculator::*;
pub use time_processor::*;

/// # Errors
///
/// Will return `Err` if the espn api call fails
pub async fn go_get_espn_data(
    scores: Vec<Scores>,
    year: i32,
    event_id: i32,
) -> Result<Vec<Scores>, Box<dyn std::error::Error>> {
    let json_responses = get_espn_data_parallel(&scores, year, event_id).await?;
    let statistics = process_json_to_statistics(&json_responses)?;
    merge_statistics_with_scores(&statistics, &scores)
}
