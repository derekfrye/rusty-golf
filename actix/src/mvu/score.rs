use maud::Markup;
use rusty_golf_core::storage::Storage;

use crate::controller::score::data_service::get_data_for_scores_page;
use crate::model::{RefreshSource, ScoreData, ScoresAndLastRefresh};
use crate::view::score::{
    render_scores_template_pure, scores_and_last_refresh_to_line_score_tables,
};
use rusty_golf_core::error::CoreError;
use rusty_golf_core::score::decode_score_request;
use std::collections::HashMap;
use std::hash::BuildHasher;

#[derive(Debug, Clone)]
pub struct ScoreModel {
    pub event_id: i32,
    pub year: i32,
    pub use_cache: bool,
    pub expanded: bool,
    pub want_json: bool,
    pub cache_max_age: i64,
    pub data: Option<ScoreData>,
    pub markup: Option<Markup>,
    pub error: Option<CoreError>,
    pub from_db_scores: Option<ScoresAndLastRefresh>,
    pub global_step_factor: Option<f32>,
    pub player_step_factors: Option<HashMap<(i64, String), f32>>,
}

impl ScoreModel {
    #[must_use]
    pub fn new(
        event_id: i32,
        year: i32,
        use_cache: bool,
        expanded: bool,
        want_json: bool,
        cache_max_age: i64,
    ) -> Self {
        Self {
            event_id,
            year,
            use_cache,
            expanded,
            want_json,
            cache_max_age,
            data: None,
            markup: None,
            error: None,
            from_db_scores: None,
            global_step_factor: None,
            player_step_factors: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    PageLoad,
    ScoresLoaded(ScoreData),
    EventConfigLoaded(f32),
    PlayerFactorsLoaded(HashMap<(i64, String), f32>),
    DbScoresLoaded(ScoresAndLastRefresh),
    Rendered(Markup),
    Failed(CoreError),
}

#[derive(Debug, Clone)]
pub enum Effect {
    LoadScores,
    LoadEventConfig,
    LoadPlayerFactors,
    LoadDbScores,
    RenderTemplate,
}

pub fn update(model: &mut ScoreModel, msg: Msg) -> Vec<Effect> {
    match msg {
        Msg::PageLoad => vec![
            Effect::LoadScores,
            Effect::LoadEventConfig,
            Effect::LoadPlayerFactors,
            Effect::LoadDbScores,
        ],
        Msg::ScoresLoaded(data) => {
            model.data = Some(data);
            if model.want_json {
                vec![]
            } else if model.from_db_scores.is_some()
                && model.global_step_factor.is_some()
                && model.player_step_factors.is_some()
            {
                vec![Effect::RenderTemplate]
            } else {
                vec![]
            }
        }
        Msg::EventConfigLoaded(step) => {
            model.global_step_factor = Some(step);
            if !model.want_json
                && model.data.is_some()
                && model.from_db_scores.is_some()
                && model.player_step_factors.is_some()
            {
                vec![Effect::RenderTemplate]
            } else {
                vec![]
            }
        }
        Msg::PlayerFactorsLoaded(factors) => {
            model.player_step_factors = Some(factors);
            if !model.want_json
                && model.data.is_some()
                && model.from_db_scores.is_some()
                && model.global_step_factor.is_some()
            {
                vec![Effect::RenderTemplate]
            } else {
                vec![]
            }
        }
        Msg::DbScoresLoaded(from_db_scores) => {
            model.from_db_scores = Some(from_db_scores);
            if !model.want_json
                && model.data.is_some()
                && model.player_step_factors.is_some()
                && model.global_step_factor.is_some()
            {
                vec![Effect::RenderTemplate]
            } else {
                vec![]
            }
        }
        Msg::Rendered(markup) => {
            model.markup = Some(markup);
            vec![]
        }
        Msg::Failed(e) => {
            model.error = Some(e);
            vec![]
        }
    }
}

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
        Effect::RenderTemplate => {
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
    }
}

/// Parse query params into a `ScoreModel`, computing `cache_max_age` from event config.
///
/// # Errors
///
/// Returns `CoreError::Other` with human-readable messages for missing or invalid params.
pub async fn decode_request_to_model<S: BuildHasher>(
    query: &HashMap<String, String, S>,
    storage: &dyn Storage,
) -> Result<ScoreModel, CoreError> {
    let mut owned_query = HashMap::new();
    for (key, value) in query {
        owned_query.insert(key.clone(), value.clone());
    }
    decode_score_request(&owned_query, storage, |req, cache_max_age| {
        ScoreModel::new(
            req.event_id,
            req.year,
            req.use_cache,
            req.expanded,
            req.want_json,
            cache_max_age,
        )
    })
    .await
}
