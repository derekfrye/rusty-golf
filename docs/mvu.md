# MVU Architecture (Server‑Side)

This project uses a request‑driven, server‑side MVU (Model–View–Update) pattern for the scores page. Views are pure; all IO happens in effects executed by the controller layer.

## Concepts
- Model: typed state for a single request. Example: `src/mvu/score.rs::ScoreModel` holds `event_id`, `year`, flags, `ScoreData`, and precomputed view deps.
- Msg: domain events that drive state changes (no IO). Example: `PageLoad`, `ScoresLoaded(ScoreData)`, `ViewDepsLoaded { .. }`, `Rendered(Markup)`, `Failed(String)`.
- Update: pure function `update(model, msg) -> Vec<Effect>` that mutates the model and returns effects to run.
- Effects: declarative IO tasks (DB/API). Example: `LoadScores`, `LoadViewDeps`, `RenderTemplate`.

## Flow (per request)
1. Handler maps HTTP request → initial `Msg` (e.g., `PageLoad`).
2. Run `update` to produce effects, then execute effects asynchronously and feed resulting `Msg`s back into `update` until no effects remain.
3. Render: `Rendered(Markup)` is set on the model and returned.

See: `src/mvu/score.rs` and `src/controller/score/http_handlers.rs`.

## Pure Views
- Top‑level: `render_scores_template_pure` in `src/view/score/template.rs`.
- Chart helpers: `preprocess_golfer_data_pure` and `render_drop_down_bar_pure` in `src/view/score/chart.rs`.
- All view fns are IO‑free; they accept only data computed by effects.

## Effects and IO
- Data fetch: `get_data_for_scores_page` (scores + ESPN/cache) → `ScoresLoaded`.
- View deps: `get_scores_from_db`, `get_event_details`, `get_player_step_factors` → `ViewDepsLoaded`.
- Offline fallback: on ESPN HTTP failure, `fetch_scores_from_espn` loads `tests/test3_espn_json_responses.json` and persists via normal DB path. No flags needed.

## Runtime (next step)
- Extract a tiny runtime: `src/mvu/runtime.rs`
  - `run(model, init_msg, deps) -> Result<Model, AppError>`: drains effects using a loop, returns the settled model.
  - Keeps handlers thin and consistent across endpoints.

## Endpoints & Partials (incremental)
- Keep `/scores` as today (full page or JSON).
- Add partials (no UX change yet): `/scores/summary`, `/scores/chart`, `/scores/linescore` that map to pure renderers for htmx integration later.

## Testing
- Unit: table‑driven tests for `update` (pure), and effect sequencing with a mock effect runner.
- Integration: existing nextest suite remains; add golden tests for partials using stable inputs.
- Run: `cargo nextest run --no-fail-fast`.

## Guidelines for new features
- Define `Model/Msg/Effect` in a module under `src/mvu/`.
- Keep `update` pure; isolate IO in effects.
- Keep views pure; accept data only.
- Prefer request‑driven flows; avoid background tick loops (client can use htmx later if needed).
