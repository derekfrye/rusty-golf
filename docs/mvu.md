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
- Offline fallback: on ESPN HTTP failure, `fetch_scores_from_espn` loads `tests/test03_espn_json_responses.json` and persists via normal DB path. No flags needed.

## Runtime (implemented)
- A small runtime exists at `src/mvu/runtime.rs`.
  - `run_score(model, init_msg, deps) -> Result<(), String>` drains effects and records failures into the model.
  - Handlers use this to stay thin and consistent.

## Endpoints & Partials (implemented)
- `/scores` returns full HTML (or JSON) using the MVU flow.
- Partials exposed: `/scores/summary`, `/scores/chart`, `/scores/linescore` render pure fragments.
- The top‑level template includes htmx containers (`hx-get` + `hx-trigger=load`) that request these partials on page load. Without JS, SSR content is still rendered.

## Testing
- Unit: table‑driven tests for `update` (pure), and effect sequencing with a mock effect runner.
- Integration: existing nextest suite remains; add golden tests for partials using stable inputs.
- Run: `cargo nextest run --no-fail-fast`.

## Guidelines for new features
- Define `Model/Msg/Effect` in a module under `src/mvu/`.
- Keep `update` pure; isolate IO in effects.
- Keep views pure; accept data only.
- Prefer request‑driven flows; avoid background tick loops (client can use htmx later if needed).

## Design TODOs
// Split effects — DONE
// Typed error handling + structured logs — DONE
// Request decoding helper — DONE
- Avoid duplicate IO for partials — Deferred: see Per‑Request IO Strategy.
- Extend MVU to mutating flows: add Msgs/Effects for actions like `SetStepFactor`, with DB writes and optimistic UI update.
- Add unit tests for the runtime loop and partial endpoints (golden HTML).

## Per‑Request IO Strategy
- Considered: Conditional caching via ETag/Last‑Modified using an event version derived from DB timestamps to short‑circuit unchanged partials (304).
- Decision: Do not implement now.
  - Rationale: SQLite reads are already fast and load is low; adding cache/etag handling increases complexity and risks subtle edge cases with little practical benefit.
  - Tradeoff accepted: Read from DB on each request for correctness and simplicity.
- If needs change: introduce a small helper to compute an event version (e.g., min/max `ins_ts` + step‑factor version) and attach `ETag` headers to partial responses.
- Typed error handling: introduce an `AppError` enum and structured logs for Msg/Effect transitions.
- Request decoding: add a small helper to parse query params → `Msg` to keep handlers minimal and consistent.
- Avoid duplicate IO for partials: consider lightweight caching of per-request results or ETags to prevent re-fetch across partial endpoints.
- Extend MVU to mutating flows: add Msgs/Effects for actions like `SetStepFactor`, with DB writes and optimistic UI update.
- Add unit tests for the runtime loop and partial endpoints (golden HTML).
