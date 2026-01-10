use maud::Markup;
use crate::model::{ScoreData, ScoresAndLastRefresh};
use rusty_golf_core::error::CoreError;
use std::collections::HashMap;

mod score_decode;
mod score_effects;

pub use score_decode::decode_request_to_model;
pub use score_effects::{Deps, run_effect};

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
