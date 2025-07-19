use maud::Markup;
use sql_middleware::middleware::ConfigAndPool;

use crate::model::{RefreshSource, ScoreData, get_scores_from_db};
use crate::view::score::types::RefreshData;
use crate::view::score::{
    render_drop_down_bar, render_line_score_tables, render_scoreboard, render_summary_scores,
    scores_and_last_refresh_to_line_score_tables,
};


/// # Errors
///
/// Will return `Err` if the database query fails
pub async fn render_scores_template(
    data: &ScoreData,
    expanded: bool,
    config_and_pool: &ConfigAndPool,
    event_id: i32,
) -> Result<Markup, Box<dyn std::error::Error>> {
    let summary_scores_x =
        crate::controller::score::group_by_bettor_name_and_round(&data.score_struct);
    let detailed_scores =
        crate::controller::score::group_by_bettor_golfer_round(&data.score_struct);

    let golfer_scores_for_line_score_render =
        get_scores_from_db(config_and_pool, event_id, RefreshSource::Db).await?;

    let bettor_struct =
        scores_and_last_refresh_to_line_score_tables(&golfer_scores_for_line_score_render);

    let refresh_data = RefreshData {
        last_refresh: data.last_refresh.clone(),
        last_refresh_source: data.last_refresh_source.clone(),
    };

    Ok(maud::html! {
        (render_scoreboard(data))
        @if expanded {
            (render_summary_scores(&summary_scores_x))
        }
        (render_drop_down_bar(&summary_scores_x, &detailed_scores, config_and_pool, event_id).await?)
        (render_line_score_tables(&bettor_struct, refresh_data))
    })
}
