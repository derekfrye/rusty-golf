use rusty_golf_core::error::CoreError;
use rusty_golf_core::storage::Storage;

use crate::controller::score::data_service::get_data_for_scores_page;
use crate::model::RefreshSource;
use crate::mvu::score::{Effect, Msg, ScoreModel};
use crate::view::score::{
    render_scores_template_pure, scores_and_last_refresh_to_line_score_tables,
};

#[derive(Clone, Copy)]
pub struct Deps<'a> {
    pub storage: &'a dyn Storage,
}

pub async fn run_effect(effect: Effect, model: &ScoreModel, deps: Deps<'_>) -> Msg {
    match effect {
        Effect::LoadScores => {
            match get_data_for_scores_page(
                model.event_id,
                model.year,
                model.use_cache,
                deps.storage,
                model.cache_max_age,
            )
            .await
            {
                Ok(data) => Msg::ScoresLoaded(data),
                Err(e) => Msg::Failed(e),
            }
        }
        Effect::LoadEventConfig => match deps.storage.get_event_details(model.event_id).await {
            Ok(event_details) => Msg::EventConfigLoaded(event_details.score_view_step_factor),
            Err(e) => Msg::Failed(CoreError::from(e)),
        },
        Effect::LoadPlayerFactors => {
            match deps.storage.get_player_step_factors(model.event_id).await {
                Ok(factors) => Msg::PlayerFactorsLoaded(factors),
                Err(e) => Msg::Failed(CoreError::from(e)),
            }
        }
        Effect::LoadDbScores => {
            match deps
                .storage
                .get_scores(model.event_id, RefreshSource::Db)
                .await
            {
                Ok(from_db_scores) => Msg::DbScoresLoaded(from_db_scores),
                Err(e) => Msg::Failed(CoreError::from(e)),
            }
        }
        Effect::RenderTemplate => render_template(model),
    }
}

fn render_template(model: &ScoreModel) -> Msg {
    if let (Some(data), Some(from_db), Some(global_step), Some(player_factors)) = (
        model.data.as_ref(),
        model.from_db_scores.as_ref(),
        model.global_step_factor,
        model.player_step_factors.as_ref(),
    ) {
        let bettor_struct = scores_and_last_refresh_to_line_score_tables(from_db);
        let markup = render_scores_template_pure(
            data,
            model.expanded,
            &bettor_struct,
            global_step,
            player_factors,
            model.event_id,
            model.year,
            model.use_cache,
        );
        Msg::Rendered(markup)
    } else {
        Msg::Failed(CoreError::Other("Render requested without deps".into()))
    }
}
