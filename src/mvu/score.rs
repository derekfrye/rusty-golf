use maud::Markup;
use sql_middleware::middleware::ConfigAndPool;

use crate::controller::score::data_service::get_data_for_scores_page;
use crate::model::{ScoreData, RefreshSource, ScoresAndLastRefresh};
use crate::model::event::get_event_details;
use crate::model::golfer::get_player_step_factors;
use crate::model::database_read::get_scores_from_db;
use crate::view::score::{render_scores_template_pure, scores_and_last_refresh_to_line_score_tables};
use std::collections::HashMap;

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
    pub error: Option<String>,
    pub from_db_scores: Option<ScoresAndLastRefresh>,
    pub global_step_factor: Option<f32>,
    pub player_step_factors: Option<HashMap<(i64, String), f32>>,
}

impl ScoreModel {
    pub fn new(event_id: i32, year: i32, use_cache: bool, expanded: bool, want_json: bool, cache_max_age: i64) -> Self {
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
    ViewDepsLoaded {
        from_db_scores: ScoresAndLastRefresh,
        global_step_factor: f32,
        player_step_factors: HashMap<(i64, String), f32>,
    },
    Rendered(Markup),
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum Effect {
    LoadScores,
    LoadViewDeps,
    RenderTemplate,
}

pub fn update(model: &mut ScoreModel, msg: Msg) -> Vec<Effect> {
    match msg {
        Msg::PageLoad => vec![Effect::LoadScores, Effect::LoadViewDeps],
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
        Msg::ViewDepsLoaded { from_db_scores, global_step_factor, player_step_factors } => {
            model.from_db_scores = Some(from_db_scores);
            model.global_step_factor = Some(global_step_factor);
            model.player_step_factors = Some(player_step_factors);
            if !model.want_json && model.data.is_some() {
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
                Err(e) => Msg::Failed(e.to_string()),
            }
        }
        Effect::LoadViewDeps => {
            // Gather all IO needed for a pure render
            let from_db = get_scores_from_db(deps.config_and_pool, model.event_id, RefreshSource::Db).await;
            let evt = get_event_details(deps.config_and_pool, model.event_id).await;
            let player = get_player_step_factors(deps.config_and_pool, model.event_id).await;

            match (from_db, evt, player) {
                (Ok(from_db_scores), Ok(event_details), Ok(player_step_factors)) => Msg::ViewDepsLoaded {
                    from_db_scores,
                    global_step_factor: event_details.score_view_step_factor,
                    player_step_factors,
                },
                (a, b, c) => {
                    let err = format!(
                        "view deps error: db={:?} evt={:?} player={:?}",
                        a.as_ref().err(), b.as_ref().err(), c.as_ref().err()
                    );
                    Msg::Failed(err)
                }
            }
        }
        Effect::RenderTemplate => {
            if let (Some(ref data), Some(ref from_db), Some(global_step), Some(ref player_factors)) = (
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
                Msg::Failed("Render requested without deps".into())
            }
        }
    }
}
