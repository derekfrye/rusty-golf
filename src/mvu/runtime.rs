use crate::mvu::score::{update, Deps, Msg, ScoreModel};

/// Runs the MVU loop for the scores model: seeds with `init_msg` and drains effects.
pub async fn run_score(
    model: &mut ScoreModel,
    init_msg: Msg,
    deps: Deps<'_>,
) -> Result<(), String> {
    let mut effects = update(model, init_msg);
    while let Some(effect) = effects.pop() {
        let msg = super::score::run_effect(effect, &model, deps).await;
        match msg {
            Msg::Failed(e) => {
                // Record failure and stop the loop.
                update(model, Msg::Failed(e.clone()));
                return Err(e);
            }
            other => {
                let next = update(model, other);
                effects.extend(next);
            }
        }
    }
    Ok(())
}
