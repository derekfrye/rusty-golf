use maud::Markup;
use sql_middleware::middleware::ConfigAndPool;

use super::error::AppError;
use crate::controller::score::data_service::get_data_for_scores_page;
use crate::model::database_read::get_scores_from_db;
use crate::model::event::get_event_details;
use crate::model::golfer::get_player_step_factors;
use crate::model::{RefreshSource, ScoreData, ScoresAndLastRefresh};
use crate::view::score::{
    render_scores_template_pure, scores_and_last_refresh_to_line_score_tables,
};
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
    pub error: Option<AppError>,
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
    Failed(AppError),
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
    pub config_and_pool: &'a ConfigAndPool,
}

pub async fn run_effect(effect: Effect, model: &ScoreModel, deps: Deps<'_>) -> Msg {
    match effect {
        Effect::LoadScores => {
            match get_data_for_scores_page(
                model.event_id,
                model.year,
                model.use_cache,
                deps.config_and_pool,
                model.cache_max_age,
            )
            .await
            {
                Ok(data) => Msg::ScoresLoaded(data),
                Err(e) => Msg::Failed(AppError::from(e.to_string())),
            }
        }
        Effect::LoadEventConfig => {
            match get_event_details(deps.config_and_pool, model.event_id).await {
                Ok(event_details) => Msg::EventConfigLoaded(event_details.score_view_step_factor),
                Err(e) => Msg::Failed(AppError::from(e)),
            }
        }
        Effect::LoadPlayerFactors => {
            match get_player_step_factors(deps.config_and_pool, model.event_id).await {
                Ok(factors) => Msg::PlayerFactorsLoaded(factors),
                Err(e) => Msg::Failed(AppError::from(e)),
            }
        }
        Effect::LoadDbScores => {
            match get_scores_from_db(deps.config_and_pool, model.event_id, RefreshSource::Db).await
            {
                Ok(from_db_scores) => Msg::DbScoresLoaded(from_db_scores),
                Err(e) => Msg::Failed(AppError::from(e)),
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
                Msg::Failed(AppError::Other("Render requested without deps".into()))
            }
        }
    }
}

/// Parse query params into a `ScoreModel`, computing `cache_max_age` from event config.
///
/// # Errors
///
/// Returns `AppError::Other` with human-readable messages for missing or invalid params.
pub async fn decode_request_to_model<S: BuildHasher>(
    query: &HashMap<String, String, S>,
    config_and_pool: &ConfigAndPool,
) -> Result<ScoreModel, AppError> {
    let event_id: i32 = query
        .get("event")
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| AppError::Other("espn event parameter is required".into()))?;

    let year: i32 = query
        .get("yr")
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| AppError::Other("yr (year) parameter is required".into()))?;

    let cache = !matches!(query.get("cache").map(String::as_str), Some("0"));

    let want_json = match query.get("json").map(String::as_str) {
        Some("1") => true,
        Some("0") | None => false,
        Some(other) => other.parse().unwrap_or(false),
    };

    let expanded = match query.get("expanded").map(String::as_str) {
        Some("1") => true,
        Some("0") | None => false,
        Some(other) => other.parse().unwrap_or(false),
    };

    let cache_max_age: i64 = match get_event_details(config_and_pool, event_id).await {
        Ok(event_details) => match event_details.refresh_from_espn {
            1 => 99,
            _ => 0,
        },
        Err(_) => 0,
    };

    Ok(ScoreModel::new(
        event_id,
        year,
        cache,
        expanded,
        want_json,
        cache_max_age,
    ))
}
