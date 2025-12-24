use rusty_golf_core::error::CoreError;
use crate::mvu::score::{Deps, Msg, ScoreModel, update};
use serde_json::json;

/// Runs the MVU loop for the scores model: seeds with `init_msg` and drains effects.
///
/// # Errors
///
/// Returns an error if the `update` loop surfaces a `CoreError`.
pub async fn run_score(
    model: &mut ScoreModel,
    init_msg: Msg,
    deps: Deps<'_>,
) -> Result<(), CoreError> {
    let mut effects = update(model, init_msg);
    while let Some(effect) = effects.pop() {
        if cfg!(debug_assertions) {
            eprintln!(
                "{}",
                json!({"mvu":"effect_start","effect": format!("{effect:?}")})
            );
        }
        let msg = super::score::run_effect(effect, model, deps).await;
        if cfg!(debug_assertions) {
            eprintln!("{}", json!({"mvu":"effect_done","msg": format!("{msg:?}")}));
        }
        match msg {
            Msg::Failed(e) => {
                update(model, Msg::Failed(e.clone()));
                return Err(e);
            }
            other => {
                let next = update(model, other);
                if cfg!(debug_assertions) {
                    eprintln!(
                        "{}",
                        json!({"mvu":"update","queued_effects": next.iter().map(|x| format!("{x:?}")).collect::<Vec<_>>()})
                    );
                }
                effects.extend(next);
            }
        }
    }
    Ok(())
}
